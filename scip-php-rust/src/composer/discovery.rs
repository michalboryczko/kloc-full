//! File discovery for PHP projects using Composer autoload configuration.
//!
//! Discovers .php files in project source directories and vendor directories.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::types::AutoloadConfig;

/// Walk directories and collect all `.php` files.
///
/// Follows symlinks: false. Excludes hidden directories (starting with `.`).
/// Returns files sorted by path for deterministic ordering.
pub fn discover_php_files(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        let walker = WalkDir::new(dir)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| !is_hidden(e));

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            if is_php_file(entry.path()) {
                files.push(entry.path().to_owned());
            }
        }
    }

    files.sort();
    files.dedup();
    files
}

/// Determine project source directories from the autoload configuration.
///
/// Collects all directories referenced in PSR-4, PSR-0, classmap, and files
/// from both `autoload` and `autoload-dev` sections.
pub fn project_source_dirs(
    autoload: Option<&AutoloadConfig>,
    autoload_dev: Option<&AutoloadConfig>,
    project_root: &Path,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for config in [autoload, autoload_dev].iter().copied().flatten() {
        // PSR-4 directories
        if let Some(psr4) = &config.psr4 {
            for (_prefix, paths) in psr4 {
                for path in paths.to_vec() {
                    let dir = project_root.join(path.trim_end_matches('/'));
                    if dir.exists() {
                        dirs.push(dir);
                    }
                }
            }
        }

        // PSR-0 directories
        if let Some(psr0) = &config.psr0 {
            for (_prefix, paths) in psr0 {
                for path in paths.to_vec() {
                    let dir = project_root.join(path.trim_end_matches('/'));
                    if dir.exists() {
                        dirs.push(dir);
                    }
                }
            }
        }

        // Classmap directories/files
        if let Some(classmap) = &config.classmap {
            for path in classmap {
                let full = project_root.join(path.trim_end_matches('/'));
                if full.exists() {
                    dirs.push(full);
                }
            }
        }
    }

    dirs.sort();
    dirs.dedup();
    dirs
}

/// Collect vendor directories that contain PHP files from installed packages.
///
/// Given package install paths, returns the list of directories to scan.
pub fn vendor_source_dirs(package_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for path in package_paths {
        if path.exists() && path.is_dir() {
            dirs.push(path.clone());
        }
    }

    dirs.sort();
    dirs.dedup();
    dirs
}

/// Check if a directory entry is hidden (starts with '.').
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Project source files
        fs::create_dir_all(root.join("src/Models")).unwrap();
        fs::write(root.join("src/Models/User.php"), "<?php class User {}").unwrap();
        fs::write(root.join("src/Models/Post.php"), "<?php class Post {}").unwrap();

        fs::create_dir_all(root.join("src/Services")).unwrap();
        fs::write(
            root.join("src/Services/UserService.php"),
            "<?php class UserService {}",
        )
        .unwrap();

        // Test files
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("tests/UserTest.php"), "<?php class UserTest {}").unwrap();

        // Non-PHP file
        fs::write(root.join("src/readme.md"), "# Readme").unwrap();

        // Hidden directory
        fs::create_dir_all(root.join("src/.hidden")).unwrap();
        fs::write(root.join("src/.hidden/secret.php"), "<?php").unwrap();

        dir
    }

    #[test]
    fn test_discover_php_files_single_dir() {
        let dir = setup_project();
        let files = discover_php_files(&[dir.path().join("src")]);

        assert_eq!(files.len(), 3);
        for f in &files {
            assert_eq!(f.extension().unwrap(), "php");
        }
    }

    #[test]
    fn test_discover_php_files_multiple_dirs() {
        let dir = setup_project();
        let files = discover_php_files(&[dir.path().join("src"), dir.path().join("tests")]);

        assert_eq!(files.len(), 4); // 3 src + 1 test
    }

    #[test]
    fn test_discover_php_files_excludes_hidden() {
        let dir = setup_project();
        let files = discover_php_files(&[dir.path().join("src")]);

        for f in &files {
            assert!(
                !f.to_str().unwrap().contains(".hidden"),
                "hidden file found: {:?}",
                f
            );
        }
    }

    #[test]
    fn test_discover_php_files_nonexistent_dir() {
        let files = discover_php_files(&[PathBuf::from("/nonexistent/path")]);
        assert!(files.is_empty());
    }

    #[test]
    fn test_discover_php_files_sorted_and_deduped() {
        let dir = setup_project();
        let files = discover_php_files(&[
            dir.path().join("src"),
            dir.path().join("src"), // duplicate
        ]);

        // Should be deduped
        let mut unique = files.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(files, unique);
    }

    #[test]
    fn test_project_source_dirs() {
        let dir = setup_project();
        let root = dir.path();

        let config = AutoloadConfig {
            psr4: Some(
                [("App\\".to_string(), super::super::types::StringOrVec::Single("src/".to_string()))]
                    .into_iter()
                    .collect(),
            ),
            psr0: None,
            classmap: None,
            files: None,
        };

        let dev_config = AutoloadConfig {
            psr4: Some(
                [(
                    "Tests\\".to_string(),
                    super::super::types::StringOrVec::Single("tests/".to_string()),
                )]
                .into_iter()
                .collect(),
            ),
            psr0: None,
            classmap: None,
            files: None,
        };

        let dirs = project_source_dirs(Some(&config), Some(&dev_config), root);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&root.join("src")));
        assert!(dirs.contains(&root.join("tests")));
    }

    #[test]
    fn test_project_source_dirs_empty() {
        let dir = setup_project();
        let dirs = project_source_dirs(None, None, dir.path());
        assert!(dirs.is_empty());
    }

    #[test]
    fn test_vendor_source_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("vendor/psr/log")).unwrap();
        fs::create_dir_all(root.join("vendor/monolog/monolog")).unwrap();

        let paths = vec![
            root.join("vendor/psr/log"),
            root.join("vendor/monolog/monolog"),
            root.join("vendor/nonexistent/pkg"), // doesn't exist
        ];

        let dirs = vendor_source_dirs(&paths);
        assert_eq!(dirs.len(), 2);
    }
}
