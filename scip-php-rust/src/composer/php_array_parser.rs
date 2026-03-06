//! Regex-based extraction of PHP array entries from Composer-generated autoload files.
//!
//! Composer generates PHP files like `autoload_classmap.php`, `autoload_psr4.php`, etc.
//! These contain PHP array syntax mapping keys to paths using `$baseDir` and `$vendorDir`.

use regex::Regex;
use std::path::{Path, PathBuf};

/// Extract a flat key-value map from a PHP array file.
///
/// Matches lines like:
/// ```php
/// 'App\\Models\\User' => $baseDir . '/src/Models/User.php',
/// 'Psr\\Log\\LoggerInterface' => $vendorDir . '/psr/log/src/LoggerInterface.php',
/// ```
///
/// PHP uses double backslashes in single-quoted strings to represent a literal backslash,
/// so `'App\\Models\\User'` in PHP source means the string `App\Models\User`.
pub fn extract_flat_map(content: &str, base_dir: &Path, vendor_dir: &Path) -> Vec<(String, PathBuf)> {
    let re = Regex::new(r"'((?:[^'\\]|\\.)*)'\s*=>\s*\$(baseDir|vendorDir)\s*\.\s*'([^']*)'")
        .expect("invalid regex");

    let mut entries = Vec::new();
    for cap in re.captures_iter(content) {
        // Unescape PHP backslash escaping: \\\\ in PHP source -> \\ in captured text -> \ in meaning
        let key = cap[1].replace("\\\\", "\\");
        let dir_var = &cap[2];
        let rel_path = &cap[3];

        let dir = match dir_var {
            "baseDir" => base_dir,
            "vendorDir" => vendor_dir,
            _ => continue,
        };

        // Strip leading '/' from the relative path
        let clean_path = rel_path.trim_start_matches('/');
        let full_path = dir.join(clean_path);
        entries.push((key, full_path));
    }

    entries
}

/// Extract a prefix-to-directories map from a PHP PSR-4 or PSR-0 array file.
///
/// Matches structures like:
/// ```php
/// 'App\\' => array($baseDir . '/src'),
/// 'Psr\\Log\\' => array($vendorDir . '/psr/log/src'),
/// ```
///
/// Or array syntax with multiple directories:
/// ```php
/// 'App\\' => array($baseDir . '/src', $baseDir . '/lib'),
/// ```
pub fn extract_prefix_map(
    content: &str,
    base_dir: &Path,
    vendor_dir: &Path,
) -> Vec<(String, Vec<PathBuf>)> {
    // Match a prefix key followed by array(...) or [...]
    let prefix_re =
        Regex::new(r"'((?:[^'\\]|\\.)*)'\s*=>\s*(?:array\s*\(|\[)((?:[^\])]*(?:\)|\])))")
            .expect("invalid regex");
    let path_re = Regex::new(r"\$(baseDir|vendorDir)\s*\.\s*'([^']*)'")
        .expect("invalid regex");

    let mut entries: Vec<(String, Vec<PathBuf>)> = Vec::new();

    for cap in prefix_re.captures_iter(content) {
        let key = cap[1].replace("\\\\", "\\");
        let array_body = &cap[2];

        let mut dirs = Vec::new();
        for path_cap in path_re.captures_iter(array_body) {
            let dir_var = &path_cap[1];
            let rel_path = &path_cap[2];

            let dir = match dir_var {
                "baseDir" => base_dir,
                "vendorDir" => vendor_dir,
                _ => continue,
            };

            let clean_path = rel_path.trim_start_matches('/');
            dirs.push(dir.join(clean_path));
        }

        if !dirs.is_empty() {
            entries.push((key, dirs));
        }
    }

    entries
}

/// Extract paths from a simple PHP array of `$baseDir . '/path'` entries.
///
/// Used for autoload_files.php which has no keys, just:
/// ```php
/// $baseDir . '/vendor/autoload.php',
/// $vendorDir . '/some/file.php',
/// ```
pub fn extract_file_list(content: &str, base_dir: &Path, vendor_dir: &Path) -> Vec<PathBuf> {
    let re = Regex::new(r"\$(baseDir|vendorDir)\s*\.\s*'([^']*)'").expect("invalid regex");
    let mut files = Vec::new();

    for cap in re.captures_iter(content) {
        let dir_var = &cap[1];
        let rel_path = &cap[2];

        let dir = match dir_var {
            "baseDir" => base_dir,
            "vendorDir" => vendor_dir,
            _ => continue,
        };

        let clean_path = rel_path.trim_start_matches('/');
        files.push(dir.join(clean_path));
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_flat_map_basedir() {
        let content = r#"<?php
return array(
    'App\\Models\\User' => $baseDir . '/src/Models/User.php',
    'App\\Models\\Post' => $baseDir . '/src/Models/Post.php',
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "App\\Models\\User");
        assert_eq!(result[0].1, PathBuf::from("/project/src/Models/User.php"));
        assert_eq!(result[1].0, "App\\Models\\Post");
        assert_eq!(result[1].1, PathBuf::from("/project/src/Models/Post.php"));
    }

    #[test]
    fn test_extract_flat_map_vendordir() {
        let content = r#"<?php
return array(
    'Psr\\Log\\LoggerInterface' => $vendorDir . '/psr/log/src/LoggerInterface.php',
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "Psr\\Log\\LoggerInterface");
        assert_eq!(
            result[0].1,
            PathBuf::from("/project/vendor/psr/log/src/LoggerInterface.php")
        );
    }

    #[test]
    fn test_extract_flat_map_mixed_dirs() {
        let content = r#"<?php
return array(
    'App\\Controller' => $baseDir . '/src/Controller.php',
    'Vendor\\Lib' => $vendorDir . '/vendor-lib/src/Lib.php',
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, PathBuf::from("/project/src/Controller.php"));
        assert_eq!(
            result[1].1,
            PathBuf::from("/project/vendor/vendor-lib/src/Lib.php")
        );
    }

    #[test]
    fn test_extract_flat_map_empty() {
        let content = "<?php\nreturn array(\n);\n";
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_prefix_map() {
        let content = r#"<?php
return array(
    'App\\' => array($baseDir . '/src'),
    'Psr\\Log\\' => array($vendorDir . '/psr/log/src'),
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_prefix_map(content, base, vendor);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "App\\");
        assert_eq!(result[0].1, vec![PathBuf::from("/project/src")]);
        assert_eq!(result[1].0, "Psr\\Log\\");
        assert_eq!(
            result[1].1,
            vec![PathBuf::from("/project/vendor/psr/log/src")]
        );
    }

    #[test]
    fn test_extract_prefix_map_multiple_dirs() {
        let content = r#"<?php
return array(
    'App\\' => array($baseDir . '/src', $baseDir . '/lib'),
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_prefix_map(content, base, vendor);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "App\\");
        assert_eq!(
            result[0].1,
            vec![PathBuf::from("/project/src"), PathBuf::from("/project/lib")]
        );
    }

    #[test]
    fn test_extract_file_list() {
        let content = r#"<?php
return array(
    $baseDir . '/helpers.php',
    $vendorDir . '/some/package/bootstrap.php',
);
"#;
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_file_list(content, base, vendor);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], PathBuf::from("/project/helpers.php"));
        assert_eq!(
            result[1],
            PathBuf::from("/project/vendor/some/package/bootstrap.php")
        );
    }

    #[test]
    fn test_backslash_handling() {
        // PHP source uses \\\\ which becomes \\ in regex capture, which we normalize to \
        let content = r"'App\\Sub\\Class' => $baseDir . '/src/Sub/Class.php',";
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "App\\Sub\\Class");
    }

    #[test]
    fn test_extract_flat_map_no_leading_slash() {
        // Some composer outputs use paths without leading /
        let content = r"'SomeClass' => $baseDir . 'src/SomeClass.php',";
        let base = Path::new("/project");
        let vendor = Path::new("/project/vendor");
        let result = extract_flat_map(content, base, vendor);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, PathBuf::from("/project/src/SomeClass.php"));
    }
}
