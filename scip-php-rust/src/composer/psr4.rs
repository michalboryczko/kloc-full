//! PSR-4 and PSR-0 class-to-file resolution.
//!
//! Implements the Composer autoloading algorithms for resolving
//! fully qualified class names to file paths.

use std::path::PathBuf;

/// Resolve a fully qualified class name using PSR-4 autoloading rules.
///
/// The algorithm:
/// 1. Find the longest matching prefix in the PSR-4 map.
/// 2. Strip the prefix from the FQN.
/// 3. Convert remaining namespace separators (`\`) to directory separators (`/`).
/// 4. Append `.php`.
/// 5. Try each directory registered for the prefix.
///
/// The `psr4_prefixes` must be sorted longest-first for correct matching.
///
/// # Example
/// Given prefix `App\` -> `/project/src`, resolving `App\Models\User` yields
/// `/project/src/Models/User.php`.
pub fn resolve_psr4(fqn: &str, psr4_prefixes: &[(String, Vec<PathBuf>)]) -> Option<PathBuf> {
    let fqn = fqn.trim_start_matches('\\');

    for (prefix, dirs) in psr4_prefixes {
        if fqn.starts_with(prefix.as_str()) {
            let relative = &fqn[prefix.len()..];
            let relative_path = relative.replace('\\', "/") + ".php";

            for dir in dirs {
                let full_path = dir.join(&relative_path);
                if full_path.exists() {
                    return Some(full_path);
                }
            }
        }
    }

    None
}

/// Resolve a fully qualified class name using PSR-0 autoloading rules.
///
/// The algorithm:
/// 1. Find the longest matching prefix in the PSR-0 map.
/// 2. Convert namespace separators (`\`) to directory separators (`/`).
/// 3. In the class name portion (after the last `\`), convert `_` to `/`.
/// 4. Append `.php`.
/// 5. Try each directory registered for the prefix.
///
/// # Example
/// Given prefix `Legacy_` -> `/project/lib`, resolving `Legacy_Sub_Class` yields
/// `/project/lib/Legacy/Sub/Class.php`.
pub fn resolve_psr0(fqn: &str, psr0_prefixes: &[(String, Vec<PathBuf>)]) -> Option<PathBuf> {
    let fqn = fqn.trim_start_matches('\\');

    for (prefix, dirs) in psr0_prefixes {
        let prefix_trimmed = prefix.trim_start_matches('\\');
        if !fqn.starts_with(prefix_trimmed) {
            continue;
        }

        // For PSR-0, the entire FQN is used to build the path (prefix is NOT stripped)
        let relative_path = if let Some(last_ns_pos) = fqn.rfind('\\') {
            // Has namespace: convert \ to / for namespace part
            let namespace = fqn[..last_ns_pos].replace('\\', "/");
            // Class name: convert _ to / for PEAR-style naming
            let class_name = fqn[last_ns_pos + 1..].replace('_', "/");
            format!("{}/{}.php", namespace, class_name)
        } else {
            // No namespace: just convert _ to /
            format!("{}.php", fqn.replace('_', "/"))
        };

        for dir in dirs {
            let full_path = dir.join(&relative_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    None
}

/// Sort PSR-4 (or PSR-0) prefixes longest-first for correct matching.
///
/// Longer prefixes must be checked first so that `App\Models\` matches
/// before `App\`.
pub fn sort_prefixes_longest_first(prefixes: &mut [(String, Vec<PathBuf>)]) {
    prefixes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_psr4_project() -> (TempDir, Vec<(String, Vec<PathBuf>)>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create src/Models/User.php
        fs::create_dir_all(root.join("src/Models")).unwrap();
        fs::write(root.join("src/Models/User.php"), "<?php class User {}").unwrap();

        // Create src/Services/UserService.php
        fs::create_dir_all(root.join("src/Services")).unwrap();
        fs::write(
            root.join("src/Services/UserService.php"),
            "<?php class UserService {}",
        )
        .unwrap();

        // Create vendor/psr/log/src/LoggerInterface.php
        fs::create_dir_all(root.join("vendor/psr/log/src")).unwrap();
        fs::write(
            root.join("vendor/psr/log/src/LoggerInterface.php"),
            "<?php interface LoggerInterface {}",
        )
        .unwrap();

        let prefixes = vec![
            (
                "App\\".to_string(),
                vec![root.join("src")],
            ),
            (
                "Psr\\Log\\".to_string(),
                vec![root.join("vendor/psr/log/src")],
            ),
        ];

        (dir, prefixes)
    }

    #[test]
    fn test_resolve_psr4_project_class() {
        let (dir, mut prefixes) = setup_psr4_project();
        sort_prefixes_longest_first(&mut prefixes);

        let result = resolve_psr4("App\\Models\\User", &prefixes);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), dir.path().join("src/Models/User.php"));
    }

    #[test]
    fn test_resolve_psr4_vendor_class() {
        let (dir, mut prefixes) = setup_psr4_project();
        sort_prefixes_longest_first(&mut prefixes);

        let result = resolve_psr4("Psr\\Log\\LoggerInterface", &prefixes);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            dir.path().join("vendor/psr/log/src/LoggerInterface.php")
        );
    }

    #[test]
    fn test_resolve_psr4_not_found() {
        let (_dir, mut prefixes) = setup_psr4_project();
        sort_prefixes_longest_first(&mut prefixes);

        let result = resolve_psr4("Unknown\\Class", &prefixes);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_psr4_strips_leading_backslash() {
        let (dir, mut prefixes) = setup_psr4_project();
        sort_prefixes_longest_first(&mut prefixes);

        let result = resolve_psr4("\\App\\Models\\User", &prefixes);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), dir.path().join("src/Models/User.php"));
    }

    #[test]
    fn test_resolve_psr4_multiple_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Second dir has the file
        fs::create_dir_all(root.join("lib/Models")).unwrap();
        fs::write(root.join("lib/Models/Foo.php"), "<?php").unwrap();

        let prefixes = vec![(
            "App\\".to_string(),
            vec![root.join("src"), root.join("lib")],
        )];

        let result = resolve_psr4("App\\Models\\Foo", &prefixes);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("lib/Models/Foo.php"));
    }

    #[test]
    fn test_resolve_psr0_basic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("lib/Legacy/Sub")).unwrap();
        fs::write(root.join("lib/Legacy/Sub/Class.php"), "<?php").unwrap();

        let prefixes = vec![("Legacy_".to_string(), vec![root.join("lib")])];

        let result = resolve_psr0("Legacy_Sub_Class", &prefixes);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), root.join("lib/Legacy/Sub/Class.php"));
    }

    #[test]
    fn test_resolve_psr0_with_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("lib/Acme/Package/Sub")).unwrap();
        fs::write(root.join("lib/Acme/Package/Sub/MyClass.php"), "<?php").unwrap();

        let prefixes = vec![("Acme\\Package\\".to_string(), vec![root.join("lib")])];

        let result = resolve_psr0("Acme\\Package\\Sub\\MyClass", &prefixes);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            root.join("lib/Acme/Package/Sub/MyClass.php")
        );
    }

    #[test]
    fn test_sort_prefixes_longest_first() {
        let mut prefixes = vec![
            ("App\\".to_string(), vec![]),
            ("App\\Models\\".to_string(), vec![]),
            ("Psr\\".to_string(), vec![]),
        ];
        sort_prefixes_longest_first(&mut prefixes);

        assert_eq!(prefixes[0].0, "App\\Models\\");
        assert_eq!(prefixes[1].0, "App\\");
        assert_eq!(prefixes[2].0, "Psr\\");
    }
}
