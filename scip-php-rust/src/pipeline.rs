//! Indexing pipeline: serial and parallel modes.
//!
//! Orchestrates parsing, type collection, name resolution, and CST traversal
//! for PHP files. Supports both serial (single-thread) and parallel (rayon) modes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::composer::Composer;
use crate::indexing::{self, FileResult, IndexingContext};
use crate::parser::PhpParser;
use crate::symbol::namer::SymbolNamer;
use crate::types::TypeDatabase;
use crate::types::collector::collect_defs_from_file;
use crate::types::upper_chain;

// ═══════════════════════════════════════════════════════════════════════════════
// Single-file indexing (used by tests and serial mode)
// ═══════════════════════════════════════════════════════════════════════════════

/// Parse and index a single PHP file, returning the collected SCIP output.
pub fn index_single_file(
    file_path: &Path,
    type_db: &TypeDatabase,
    composer: &Composer,
    namer: &SymbolNamer,
    project_root: &Path,
) -> Result<FileResult> {
    let source = std::fs::read(file_path)
        .with_context(|| format!("failed to read PHP file: {:?}", file_path))?;
    let source_str = std::str::from_utf8(&source)
        .with_context(|| format!("non-UTF-8 content in file: {:?}", file_path))?;

    let mut parser = PhpParser::new();
    let parsed = parser
        .parse(source_str, file_path)
        .with_context(|| format!("failed to parse PHP file: {:?}", file_path))?;

    let mut ctx = IndexingContext::new(
        file_path,
        &parsed.source,
        type_db,
        composer,
        namer,
        project_root,
    );

    indexing::index_file(&parsed, &mut ctx);

    Ok(ctx.into_result())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 1: Parallel type collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect type definitions from all files in parallel using rayon.
///
/// Each rayon task creates its own `PhpParser` instance (Parser is not Send).
/// The `TypeDatabase` uses DashMap internally, so concurrent inserts are safe.
pub fn collect_types_parallel(
    files: &[PathBuf],
    type_db: &TypeDatabase,
) {
    files.par_iter().for_each(|file| {
        let raw_bytes = match std::fs::read(file) {
            Ok(s) => s,
            Err(_) => return,
        };
        // Strip UTF-8 BOM if present, then lossy-convert non-UTF-8 (e.g. ISO-8859-1)
        let bytes = if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &raw_bytes[3..]
        } else {
            &raw_bytes[..]
        };
        let raw = String::from_utf8_lossy(bytes);
        let source: &str = &raw;

        let mut parser = PhpParser::new();
        let parsed = match parser.parse(&source, file.as_path()) {
            Ok(p) => p,
            Err(_) => return,
        };

        collect_defs_from_file(&parsed.path, &parsed.source, &parsed.tree, type_db);
    });

    // Build transitive upper chains after all types are collected
    upper_chain::build_transitive_uppers(type_db);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 2: Parallel indexing
// ═══════════════════════════════════════════════════════════════════════════════

/// Index all files in parallel using rayon, returning unsorted results.
///
/// Each rayon task creates its own `PhpParser` and `IndexingContext`.
/// The `TypeDatabase` and `Composer` are shared read-only via `Arc`.
pub fn index_files_parallel(
    files: &[PathBuf],
    type_db: &Arc<TypeDatabase>,
    composer: &Arc<Composer>,
    namer: &Arc<SymbolNamer>,
    project_root: &Path,
) -> Vec<FileResult> {
    files.par_iter().filter_map(|file| {
        let raw = match std::fs::read(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: cannot read {}: {}", file.display(), e);
                return None;
            }
        };

        // Strip UTF-8 BOM if present, then lossy-convert non-UTF-8 (e.g. ISO-8859-1)
        let source = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &raw[3..]
        } else {
            &raw[..]
        };
        let source_lossy = String::from_utf8_lossy(source);
        let source_str: &str = &source_lossy;

        let mut parser = PhpParser::new();
        let parsed = match parser.parse(source_str, file.as_path()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: parse error in {}: {}", file.display(), e);
                return None;
            }
        };

        let mut ctx = IndexingContext::new(
            file,
            &parsed.source,
            type_db,
            composer,
            namer,
            project_root,
        );

        indexing::index_file(&parsed, &mut ctx);

        Some(ctx.into_result())
    }).collect()
}

/// Index all files serially (for testing/comparison).
pub fn index_files_serial(
    files: &[PathBuf],
    type_db: &TypeDatabase,
    composer: &Composer,
    namer: &SymbolNamer,
    project_root: &Path,
) -> Vec<FileResult> {
    files.iter().filter_map(|file| {
        index_single_file(file, type_db, composer, namer, project_root).ok()
    }).collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3: Deterministic output sorting
// ═══════════════════════════════════════════════════════════════════════════════

/// Sort file results for deterministic output.
///
/// - Sort documents by relative_path
/// - Sort occurrences within each document by (start_line, start_col, symbol)
/// - Sort symbols within each document by symbol string
pub fn sort_results_deterministic(results: &mut Vec<FileResult>) {
    results.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    for result in results.iter_mut() {
        result.occurrences.sort_by(|a, b| {
            a.range.first().cmp(&b.range.first())
                .then(a.range.get(1).cmp(&b.range.get(1)))
                .then(a.symbol.cmp(&b.symbol))
        });
        result.symbols.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Full pipeline
// ═══════════════════════════════════════════════════════════════════════════════

/// Run the full indexing pipeline: discover -> collect types -> index -> sort.
///
/// Returns sorted `Vec<FileResult>` ready for serialization.
pub fn run_pipeline(
    project_root: &Path,
    verbose: bool,
) -> Result<(Vec<FileResult>, Arc<Composer>, Arc<SymbolNamer>)> {
    use std::time::Instant;

    // Phase 0: Load Composer config
    let composer = Arc::new(
        Composer::load(project_root)
            .with_context(|| format!("failed to load composer config from {:?}", project_root))?,
    );
    let namer = Arc::new(SymbolNamer::new(
        &composer.config.name,
        &composer.config.version,
    ));

    // Phase 0: Discover PHP files
    let start = Instant::now();
    let discovered = crate::discovery::discover_php_files(project_root);
    let all_files: Vec<PathBuf> = discovered.project.iter()
        .chain(discovered.vendor.iter())
        .cloned()
        .collect();
    if verbose {
        eprintln!("Discovery: {} project + {} vendor files in {:.2}s",
            discovered.project.len(), discovered.vendor.len(), start.elapsed().as_secs_f64());
    }

    if all_files.is_empty() {
        return Ok((Vec::new(), composer, namer));
    }

    // Phase 1: Parallel type collection (all files including vendor)
    let start = Instant::now();
    let type_db = Arc::new(TypeDatabase::new());
    collect_types_parallel(&all_files, &type_db);
    if verbose {
        eprintln!("Type collection: {:.2}s ({} types)", start.elapsed().as_secs_f64(), type_db.defs.len());
    }

    // Phase 2: Parallel indexing (all files)
    let start = Instant::now();
    let mut results = index_files_parallel(&all_files, &type_db, &composer, &namer, project_root);
    if verbose {
        eprintln!("Indexing: {:.2}s ({} files)", start.elapsed().as_secs_f64(), results.len());
    }

    // Phase 3: Deterministic sorting
    sort_results_deterministic(&mut results);

    Ok((results, composer, namer))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Compile-time Send assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _assert_send() {
    fn is_send<T: Send>() {}
    is_send::<FileResult>();
    is_send::<TypeDatabase>();
    is_send::<Composer>();
    is_send::<SymbolNamer>();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project_with_file(php_source: &str) -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("src")).unwrap();
        let file_path = root.join("src/Example.php");
        fs::write(&file_path, php_source).unwrap();

        (dir, file_path)
    }

    fn setup_multi_file_project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("src")).unwrap();

        fs::write(root.join("src/Foo.php"), r#"<?php
namespace App;
class Foo {
    public function hello(): string { return "hi"; }
}
"#).unwrap();

        fs::write(root.join("src/Bar.php"), r#"<?php
namespace App;
class Bar extends Foo {
    public function world(): void {}
}
"#).unwrap();

        fs::write(root.join("src/Baz.php"), r#"<?php
namespace App;
class Baz {
    public int $x;
}
"#).unwrap();

        dir
    }

    #[test]
    fn test_single_file_pipeline() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class Example {
    public function hello(): void {
        echo "hello";
    }
}
"#;
        let (dir, file_path) = setup_project_with_file(source);
        let project_root = dir.path();

        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");

        let result = index_single_file(
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        )
        .unwrap();

        assert_eq!(result.relative_path, "src/Example.php");
        assert!(!result.occurrences.is_empty());
        assert!(!result.symbols.is_empty());
    }

    #[test]
    fn test_single_file_pipeline_with_syntax_error() {
        let source = r#"<?php
class Broken {
    public function ( {
    }
}
"#;
        let (dir, file_path) = setup_project_with_file(source);
        let project_root = dir.path();

        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");

        let result = index_single_file(
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_single_file_pipeline_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();

        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");

        let result = index_single_file(
            &dir.path().join("nonexistent.php"),
            &type_db,
            &composer,
            &namer,
            dir.path(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_single_file_pipeline_empty() {
        let (dir, file_path) = setup_project_with_file("<?php\n");
        let project_root = dir.path();

        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");

        let result = index_single_file(
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        )
        .unwrap();

        assert_eq!(result.relative_path, "src/Example.php");
        assert!(result.occurrences.is_empty());
    }

    #[test]
    fn test_parallel_type_collection() {
        let dir = setup_multi_file_project();
        let files: Vec<PathBuf> = vec![
            dir.path().join("src/Foo.php"),
            dir.path().join("src/Bar.php"),
            dir.path().join("src/Baz.php"),
        ];

        let type_db = TypeDatabase::new();
        collect_types_parallel(&files, &type_db);

        assert!(type_db.has_class("App\\Foo"));
        assert!(type_db.has_class("App\\Bar"));
        assert!(type_db.has_class("App\\Baz"));
        assert!(type_db.get_method_params("App\\Foo", "hello").is_some());
        assert_eq!(type_db.get_property_type("App\\Baz", "x").as_deref(), Some("int"));
    }

    #[test]
    fn test_parallel_indexing() {
        let dir = setup_multi_file_project();
        let project_root = dir.path();
        let files: Vec<PathBuf> = vec![
            project_root.join("src/Foo.php"),
            project_root.join("src/Bar.php"),
            project_root.join("src/Baz.php"),
        ];

        let type_db = Arc::new(TypeDatabase::new());
        collect_types_parallel(&files, &type_db);

        let composer = Arc::new(Composer::load(project_root).unwrap());
        let namer = Arc::new(SymbolNamer::new("test/project", "1.0.0"));

        let results = index_files_parallel(&files, &type_db, &composer, &namer, project_root);

        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(!r.occurrences.is_empty(), "File {} has no occurrences", r.relative_path);
        }
    }

    #[test]
    fn test_parallel_matches_serial() {
        let dir = setup_multi_file_project();
        let project_root = dir.path();
        let files: Vec<PathBuf> = vec![
            project_root.join("src/Foo.php"),
            project_root.join("src/Bar.php"),
            project_root.join("src/Baz.php"),
        ];

        let type_db = Arc::new(TypeDatabase::new());
        collect_types_parallel(&files, &type_db);

        let composer = Arc::new(Composer::load(project_root).unwrap());
        let namer = Arc::new(SymbolNamer::new("test/project", "1.0.0"));

        // Serial
        let mut serial = index_files_serial(&files, &type_db, &composer, &namer, project_root);
        sort_results_deterministic(&mut serial);

        // Parallel
        let mut parallel = index_files_parallel(&files, &type_db, &composer, &namer, project_root);
        sort_results_deterministic(&mut parallel);

        assert_eq!(serial.len(), parallel.len());
        for (s, p) in serial.iter().zip(parallel.iter()) {
            assert_eq!(s.relative_path, p.relative_path);
            assert_eq!(s.occurrences.len(), p.occurrences.len(),
                "Occurrence count mismatch in {}", s.relative_path);
            assert_eq!(s.symbols.len(), p.symbols.len(),
                "Symbol count mismatch in {}", s.relative_path);
        }
    }

    #[test]
    fn test_deterministic_sorting() {
        let mut results = vec![
            FileResult {
                relative_path: "src/Z.php".to_string(),
                occurrences: vec![],
                symbols: vec![],
                calls: vec![],
                values: vec![],
            },
            FileResult {
                relative_path: "src/A.php".to_string(),
                occurrences: vec![],
                symbols: vec![],
                calls: vec![],
                values: vec![],
            },
            FileResult {
                relative_path: "src/M.php".to_string(),
                occurrences: vec![],
                symbols: vec![],
                calls: vec![],
                values: vec![],
            },
        ];

        sort_results_deterministic(&mut results);

        assert_eq!(results[0].relative_path, "src/A.php");
        assert_eq!(results[1].relative_path, "src/M.php");
        assert_eq!(results[2].relative_path, "src/Z.php");
    }

    #[test]
    fn test_full_pipeline() {
        let dir = setup_multi_file_project();
        let (results, _, _) = run_pipeline(dir.path(), false).unwrap();

        assert_eq!(results.len(), 3);
        // Results should be sorted by path
        assert_eq!(results[0].relative_path, "src/Bar.php");
        assert_eq!(results[1].relative_path, "src/Baz.php");
        assert_eq!(results[2].relative_path, "src/Foo.php");
    }

    #[test]
    fn test_empty_file() {
        let (dir, file_path) = setup_project_with_file("<?php\n");
        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let result = index_single_file(&file_path, &type_db, &composer, &namer, dir.path()).unwrap();
        assert!(result.occurrences.is_empty());
    }

    #[test]
    fn test_syntax_error_no_panic() {
        let (dir, file_path) = setup_project_with_file("<?php\nclass Broken {\n  public function ( {\n  }\n}\n");
        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let result = index_single_file(&file_path, &type_db, &composer, &namer, dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_bom_handling() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), r#"{"name": "test/project", "version": "1.0.0"}"#).unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        // Write file with UTF-8 BOM
        let mut content = vec![0xEF, 0xBB, 0xBF]; // BOM
        content.extend_from_slice(b"<?php\nclass BomClass {}\n");
        fs::write(dir.path().join("src/Bom.php"), &content).unwrap();

        let files = vec![dir.path().join("src/Bom.php")];
        let type_db = Arc::new(TypeDatabase::new());
        collect_types_parallel(&files, &type_db);

        assert!(type_db.has_class("BomClass"), "BOM file should still be parseable");
    }

    #[test]
    fn test_php80_features() {
        let source = r#"<?php
// Named arguments, match expression, nullsafe
class User {
    public ?string $name;
    public function getName(): ?string { return $this->name; }
}
$user = new User();
$city = $user?->getName();
$label = match (1) {
    1 => 'one',
    default => 'other',
};
"#;
        let (dir, file_path) = setup_project_with_file(source);
        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let result = index_single_file(&file_path, &type_db, &composer, &namer, dir.path());
        assert!(result.is_ok(), "PHP 8.0 features should not panic");
    }

    #[test]
    fn test_php81_enum() {
        let source = r#"<?php
enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
    public function label(): string { return $this->value; }
}
"#;
        let (dir, file_path) = setup_project_with_file(source);
        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let result = index_single_file(&file_path, &type_db, &composer, &namer, dir.path());
        assert!(result.is_ok(), "PHP 8.1 enums should not panic");
    }

    #[test]
    fn test_php82_readonly_class() {
        let source = r#"<?php
readonly class ImmutableValue {
    public function __construct(
        public string $value,
        public int $count,
    ) {}
}
"#;
        let (dir, file_path) = setup_project_with_file(source);
        let type_db = TypeDatabase::new();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let result = index_single_file(&file_path, &type_db, &composer, &namer, dir.path());
        assert!(result.is_ok(), "PHP 8.2 readonly class should not panic");
    }
}
