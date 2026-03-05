//! PHP file discovery via directory traversal.
//!
//! Finds all .php files in a project, categorizing them into project files
//! and vendor files. Returns files in deterministic order (sorted by path)
//! to match PHP scip-php's file discovery order.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discovered PHP files, split into project and vendor categories.
#[derive(Debug, Default)]
pub struct DiscoveredFiles {
    /// PHP files that are part of the project (not in vendor/).
    pub project: Vec<PathBuf>,
    /// PHP files inside vendor/ (used for type resolution only).
    pub vendor: Vec<PathBuf>,
}

impl DiscoveredFiles {
    /// All files (project + vendor), sorted.
    pub fn all(&self) -> Vec<&PathBuf> {
        let mut all: Vec<&PathBuf> = self.project.iter().chain(self.vendor.iter()).collect();
        all.sort();
        all
    }
}

/// Walk the project root and collect all .php files.
///
/// - Follows symlinks: false (matches PHP behavior)
/// - Excludes hidden directories (starting with `.`)
/// - Separates vendor/ from project files
/// - Results are sorted by path for deterministic output
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use scip_php_rust::discovery::discover_php_files;
/// let files = discover_php_files(Path::new("/path/to/project"));
/// println!("Found {} project files", files.project.len());
/// println!("Found {} vendor files", files.vendor.len());
/// ```
pub fn discover_php_files(root: &Path) -> DiscoveredFiles {
    let mut result = DiscoveredFiles::default();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name() // Deterministic ordering
        .into_iter()
        .filter_entry(|e| !is_hidden(e));

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_php_file(path) {
            continue;
        }

        // Categorize: is this inside vendor/?
        if is_vendor_path(path, root) {
            result.vendor.push(path.to_owned());
        } else {
            result.project.push(path.to_owned());
        }
    }

    // Sort both lists for determinism
    result.project.sort();
    result.vendor.sort();

    result
}

/// Check if a directory entry is hidden (starts with '.').
/// The root entry (depth 0) is never considered hidden.
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Check if a file has a .php extension.
fn is_php_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("php"))
        .unwrap_or(false)
}

/// Check if a path is inside the vendor/ directory.
fn is_vendor_path(path: &Path, root: &Path) -> bool {
    // Try to strip the root prefix and check if vendor is the first component
    path.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.components().next())
        .and_then(|comp| comp.as_os_str().to_str())
        .map(|s| s == "vendor")
        .unwrap_or(false)
}

/// Compute the path relative to the project root.
/// Returns the relative path as a string with forward slashes.
pub fn relative_path(abs_path: &Path, root: &Path) -> String {
    abs_path
        .strip_prefix(root)
        .unwrap_or(abs_path)
        .to_string_lossy()
        .replace('\\', "/") // Normalize on Windows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create project files
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/App.php"), "<?php class App {}").unwrap();
        fs::write(root.join("src/User.php"), "<?php class User {}").unwrap();
        fs::write(root.join("index.php"), "<?php require 'src/App.php';").unwrap();

        // Create vendor files
        fs::create_dir_all(root.join("vendor/psr/log/src")).unwrap();
        fs::write(
            root.join("vendor/psr/log/src/LoggerInterface.php"),
            "<?php interface LoggerInterface {}",
        )
        .unwrap();

        // Non-PHP file (should be excluded)
        fs::write(root.join("composer.json"), "{}").unwrap();

        // Hidden directory (should be excluded)
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "").unwrap();

        dir
    }

    #[test]
    fn test_discovers_project_files() {
        let dir = create_test_project();
        let files = discover_php_files(dir.path());
        assert_eq!(files.project.len(), 3, "Expected 3 project PHP files");
    }

    #[test]
    fn test_separates_vendor_files() {
        let dir = create_test_project();
        let files = discover_php_files(dir.path());
        assert_eq!(files.vendor.len(), 1, "Expected 1 vendor PHP file");
        assert!(files.vendor[0].to_str().unwrap().contains("vendor"));
    }

    #[test]
    fn test_excludes_non_php() {
        let dir = create_test_project();
        let files = discover_php_files(dir.path());
        let all: Vec<_> = files.all();
        for f in &all {
            assert!(
                f.extension().map_or(false, |e| e == "php"),
                "Non-PHP file found: {:?}",
                f
            );
        }
    }

    #[test]
    fn test_excludes_hidden_directories() {
        let dir = create_test_project();
        let files = discover_php_files(dir.path());
        let all: Vec<_> = files.all();
        for f in &all {
            let path_str = f.to_str().unwrap();
            assert!(
                !path_str.contains("/.git/"),
                "Hidden dir file found: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_results_are_sorted() {
        let dir = create_test_project();
        let files = discover_php_files(dir.path());
        let mut sorted = files.project.clone();
        sorted.sort();
        assert_eq!(files.project, sorted, "Project files should be sorted");
    }

    #[test]
    fn test_relative_path() {
        let root = Path::new("/project");
        let abs = Path::new("/project/src/App.php");
        assert_eq!(relative_path(abs, root), "src/App.php");
    }

    #[test]
    fn test_is_vendor_path() {
        let root = Path::new("/project");
        assert!(is_vendor_path(
            Path::new("/project/vendor/foo/Bar.php"),
            root
        ));
        assert!(!is_vendor_path(
            Path::new("/project/src/App.php"),
            root
        ));
    }
}
