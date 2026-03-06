//! Parse Composer's installed package information.
//!
//! Reads `vendor/composer/installed.json` to discover installed packages
//! and their install paths.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

/// Information about an installed Composer package.
#[derive(Debug, Clone)]
pub struct InstalledPackageInfo {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
}

/// Serde model for installed.json (Composer v2 format).
#[derive(Debug, Deserialize)]
struct InstalledJsonV2 {
    #[serde(default)]
    packages: Vec<InstalledJsonPackage>,
}

/// Serde model for a single package entry in installed.json.
#[derive(Debug, Deserialize)]
struct InstalledJsonPackage {
    name: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "install-path", default)]
    install_path: Option<String>,
}

/// Load installed package information from vendor.
///
/// Tries to read `vendor/composer/installed.json` (Composer v2 format).
/// Returns an empty list if neither file exists.
pub fn load_installed(vendor_dir: &Path, _base_dir: &Path) -> Result<Vec<InstalledPackageInfo>> {
    let installed_json_path = vendor_dir.join("composer/installed.json");
    if installed_json_path.exists() {
        return load_from_installed_json(&installed_json_path, vendor_dir);
    }

    Ok(Vec::new())
}

/// Parse installed.json (Composer v2 format).
///
/// Composer v2 uses: `{"packages": [...], "dev": true, "dev-package-names": [...]}`
/// Composer v1 used a flat array `[...]` (also handled as fallback).
fn load_from_installed_json(
    path: &Path,
    vendor_dir: &Path,
) -> Result<Vec<InstalledPackageInfo>> {
    let content = std::fs::read_to_string(path)?;
    let composer_dir = vendor_dir.join("composer");

    // Try Composer v2 format first (object with packages key)
    if let Ok(v2) = serde_json::from_str::<InstalledJsonV2>(&content) {
        return Ok(packages_to_info(&v2.packages, &composer_dir, vendor_dir));
    }

    // Fallback: Composer v1 format (flat array)
    if let Ok(packages) = serde_json::from_str::<Vec<InstalledJsonPackage>>(&content) {
        return Ok(packages_to_info(&packages, &composer_dir, vendor_dir));
    }

    Ok(Vec::new())
}

/// Convert parsed package entries into InstalledPackageInfo with resolved paths.
fn packages_to_info(
    packages: &[InstalledJsonPackage],
    composer_dir: &Path,
    vendor_dir: &Path,
) -> Vec<InstalledPackageInfo> {
    packages
        .iter()
        .map(|pkg| {
            let install_path = match &pkg.install_path {
                Some(p) if !p.is_empty() => {
                    // install-path is relative to vendor/composer/
                    let resolved = composer_dir.join(p);
                    // Try to canonicalize; fall back to joined path
                    resolved.canonicalize().unwrap_or(resolved)
                }
                _ => {
                    // Default: vendor/<package-name>
                    vendor_dir.join(&pkg.name)
                }
            };

            InstalledPackageInfo {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                install_path,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_installed_json_v2(dir: &TempDir) -> PathBuf {
        let vendor = dir.path().join("vendor");
        let composer_dir = vendor.join("composer");
        fs::create_dir_all(&composer_dir).unwrap();

        // Create package directories
        fs::create_dir_all(vendor.join("psr/log")).unwrap();
        fs::create_dir_all(vendor.join("monolog/monolog")).unwrap();

        let content = r#"{
            "packages": [
                {
                    "name": "psr/log",
                    "version": "3.0.0",
                    "install-path": "../psr/log"
                },
                {
                    "name": "monolog/monolog",
                    "version": "3.5.0",
                    "install-path": "../monolog/monolog"
                }
            ],
            "dev": true,
            "dev-package-names": []
        }"#;
        fs::write(composer_dir.join("installed.json"), content).unwrap();

        vendor
    }

    #[test]
    fn test_load_installed_v2() {
        let dir = tempfile::tempdir().unwrap();
        let vendor = setup_installed_json_v2(&dir);
        let packages = load_installed(&vendor, dir.path()).unwrap();

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "psr/log");
        assert_eq!(packages[0].version, "3.0.0");
        assert_eq!(packages[1].name, "monolog/monolog");
        assert_eq!(packages[1].version, "3.5.0");
    }

    #[test]
    fn test_load_installed_missing() {
        let dir = tempfile::tempdir().unwrap();
        let vendor = dir.path().join("vendor");
        let packages = load_installed(&vendor, dir.path()).unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_load_installed_v1_format() {
        let dir = tempfile::tempdir().unwrap();
        let vendor = dir.path().join("vendor");
        let composer_dir = vendor.join("composer");
        fs::create_dir_all(&composer_dir).unwrap();
        fs::create_dir_all(vendor.join("foo/bar")).unwrap();

        let content = r#"[
            {
                "name": "foo/bar",
                "version": "1.0.0",
                "install-path": "../foo/bar"
            }
        ]"#;
        fs::write(composer_dir.join("installed.json"), content).unwrap();

        let packages = load_installed(&vendor, dir.path()).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "foo/bar");
    }

    #[test]
    fn test_install_path_default() {
        let dir = tempfile::tempdir().unwrap();
        let vendor = dir.path().join("vendor");
        let composer_dir = vendor.join("composer");
        fs::create_dir_all(&composer_dir).unwrap();

        let content = r#"{
            "packages": [
                {
                    "name": "foo/bar",
                    "version": "1.0.0"
                }
            ]
        }"#;
        fs::write(composer_dir.join("installed.json"), content).unwrap();

        let packages = load_installed(&vendor, dir.path()).unwrap();
        assert_eq!(packages.len(), 1);
        // Default path is vendor/<name>
        assert_eq!(packages[0].install_path, vendor.join("foo/bar"));
    }
}
