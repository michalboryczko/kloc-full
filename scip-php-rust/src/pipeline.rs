//! Single-file indexing pipeline.
//!
//! Orchestrates parsing, name resolution initialization, and CST traversal
//! for a single PHP file.

use std::path::Path;

use anyhow::{Context, Result};

use crate::composer::Composer;
use crate::indexing::{self, FileResult, IndexingContext};
use crate::parser::PhpParser;
use crate::symbol::namer::SymbolNamer;
use crate::types::TypeDatabase;

/// Parse and index a single PHP file, returning the collected SCIP output.
///
/// This is the main entry point for per-file indexing. It:
/// 1. Reads and parses the file
/// 2. Creates a fresh `IndexingContext`
/// 3. Runs the CST traversal (which initializes the name resolver internally)
/// 4. Returns the `FileResult`
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
        // Definitions are now emitted by the pipeline
        assert!(!result.occurrences.is_empty());
        assert!(!result.symbols.is_empty());
    }

    #[test]
    fn test_single_file_pipeline_with_syntax_error() {
        // Files with syntax errors should still index without panic
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
}
