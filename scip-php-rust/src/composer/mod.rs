//! Composer autoload parsing and class-to-file resolution.
//!
//! This module reads `composer.json`, `composer.lock`, and the generated
//! autoload files in `vendor/composer/` to build a complete picture of
//! the project's autoloading configuration.

pub mod classmap;
pub mod discovery;
pub mod installed;
pub mod php_array_parser;
pub mod psr4;
pub mod stubs;
pub mod types;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub use stubs::PhpStubIndex;
pub use types::*;

/// Parsed and validated Composer project configuration.
#[derive(Debug)]
pub struct ComposerConfig {
    pub name: String,
    pub version: String,
    pub psr4: HashMap<String, Vec<PathBuf>>,
    pub psr4_dev: HashMap<String, Vec<PathBuf>>,
    pub classmap_globs: Vec<String>,
    pub files: Vec<PathBuf>,
}

/// An installed Composer package with resolved paths.
#[derive(Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub autoload: AutoloadConfig,
}

/// Main Composer integration struct.
///
/// Combines composer.json configuration, installed packages, classmaps,
/// PSR-4/PSR-0 prefixes, and stub information into a single interface
/// for file discovery and class resolution.
#[derive(Debug)]
pub struct Composer {
    pub project_root: PathBuf,
    pub config: ComposerConfig,
    pub packages: Vec<InstalledPackage>,
    pub classmap: HashMap<String, PathBuf>,
    pub psr4_prefixes: Vec<(String, Vec<PathBuf>)>,
    pub psr0_prefixes: Vec<(String, Vec<PathBuf>)>,
    pub autoload_files: Vec<PathBuf>,
    pub stubs: PhpStubIndex,
}

impl Composer {
    /// Load Composer configuration from a project root directory.
    ///
    /// Reads `composer.json` and optionally `composer.lock`.
    /// Does NOT load vendor data (call `load_vendor()` separately).
    ///
    /// Gracefully handles:
    /// - Missing `composer.lock` — logs warning, continues with empty packages
    /// - Missing `vendor/` — OK for vendor-less projects
    /// - Missing stubs — uses `PhpStubIndex::empty()`
    pub fn load(project_root: &Path) -> Result<Self> {
        let composer_json_path = project_root.join("composer.json");
        let composer_json: ComposerJson = if composer_json_path.exists() {
            let content = std::fs::read_to_string(&composer_json_path)
                .with_context(|| format!("failed to read {:?}", composer_json_path))?;
            serde_json::from_str(&content)
                .with_context(|| format!("failed to parse {:?}", composer_json_path))?
        } else {
            // No composer.json — use empty defaults
            ComposerJson {
                name: None,
                version: None,
                autoload: None,
                autoload_dev: None,
                require: None,
                require_dev: None,
            }
        };

        // Build PSR-4 maps from composer.json
        let psr4 = extract_psr4_paths(composer_json.autoload.as_ref(), project_root);
        let psr4_dev = extract_psr4_paths(composer_json.autoload_dev.as_ref(), project_root);

        // Extract classmap globs
        let classmap_globs = composer_json
            .autoload
            .as_ref()
            .and_then(|a| a.classmap.clone())
            .unwrap_or_default();

        // Extract files
        let files = composer_json
            .autoload
            .as_ref()
            .and_then(|a| a.files.as_ref())
            .map(|f| f.iter().map(|p| project_root.join(p)).collect())
            .unwrap_or_default();

        let config = ComposerConfig {
            name: composer_json.name.unwrap_or_else(|| "unknown".to_string()),
            version: composer_json.version.unwrap_or_else(|| "0.0.0".to_string()),
            psr4,
            psr4_dev,
            classmap_globs,
            files,
        };

        // Read composer.lock for package info
        let packages = load_packages_from_lock(project_root);

        Ok(Composer {
            project_root: project_root.to_path_buf(),
            config,
            packages,
            classmap: HashMap::new(),
            psr4_prefixes: Vec::new(),
            psr0_prefixes: Vec::new(),
            autoload_files: Vec::new(),
            stubs: PhpStubIndex::empty(),
        })
    }

    /// Load vendor data: classmap, PSR-4/PSR-0 prefixes, autoload files, and stubs.
    ///
    /// Reads the generated autoload files in `vendor/composer/`.
    /// Safe to call even if `vendor/` does not exist (will leave fields empty).
    pub fn load_vendor(&mut self) -> Result<()> {
        let vendor_dir = self.project_root.join("vendor");

        if !vendor_dir.exists() {
            return Ok(());
        }

        // Load classmap
        self.classmap = classmap::load_classmap(&vendor_dir, &self.project_root)?;

        // Load PSR-4 prefixes from vendor/composer/autoload_psr4.php
        let psr4_path = vendor_dir.join("composer/autoload_psr4.php");
        if psr4_path.exists() {
            let content = std::fs::read_to_string(&psr4_path)?;
            self.psr4_prefixes = php_array_parser::extract_prefix_map(
                &content,
                &self.project_root,
                &vendor_dir,
            );
        }

        // Also add project-level PSR-4 prefixes from composer.json
        for (prefix, dirs) in &self.config.psr4 {
            if let Some(existing) = self
                .psr4_prefixes
                .iter_mut()
                .find(|(p, _)| p == prefix)
            {
                // Merge directories
                for d in dirs {
                    if !existing.1.contains(d) {
                        existing.1.push(d.clone());
                    }
                }
            } else {
                self.psr4_prefixes.push((prefix.clone(), dirs.clone()));
            }
        }

        // Add dev PSR-4 prefixes
        for (prefix, dirs) in &self.config.psr4_dev {
            if let Some(existing) = self
                .psr4_prefixes
                .iter_mut()
                .find(|(p, _)| p == prefix)
            {
                for d in dirs {
                    if !existing.1.contains(d) {
                        existing.1.push(d.clone());
                    }
                }
            } else {
                self.psr4_prefixes.push((prefix.clone(), dirs.clone()));
            }
        }

        // Sort prefixes longest-first for correct matching
        psr4::sort_prefixes_longest_first(&mut self.psr4_prefixes);

        // Load PSR-0 prefixes from vendor/composer/autoload_namespaces.php
        let psr0_path = vendor_dir.join("composer/autoload_namespaces.php");
        if psr0_path.exists() {
            let content = std::fs::read_to_string(&psr0_path)?;
            self.psr0_prefixes = php_array_parser::extract_prefix_map(
                &content,
                &self.project_root,
                &vendor_dir,
            );
            psr4::sort_prefixes_longest_first(&mut self.psr0_prefixes);
        }

        // Load autoload files
        let files_path = vendor_dir.join("composer/autoload_files.php");
        if files_path.exists() {
            let content = std::fs::read_to_string(&files_path)?;
            self.autoload_files =
                php_array_parser::extract_file_list(&content, &self.project_root, &vendor_dir);
        }

        // Load stubs
        self.stubs = PhpStubIndex::load_from_vendor(&vendor_dir);

        // Load installed packages info
        let installed = installed::load_installed(&vendor_dir, &self.project_root)?;
        for info in installed {
            // Check if we already have this package from composer.lock
            if !self.packages.iter().any(|p| p.name == info.name) {
                self.packages.push(InstalledPackage {
                    name: info.name,
                    version: info.version,
                    install_path: info.install_path,
                    autoload: AutoloadConfig::default(),
                });
            }
        }

        Ok(())
    }

    /// Resolve a fully qualified class name to a file path.
    ///
    /// Resolution order:
    /// 1. Classmap (exact match)
    /// 2. PSR-4 prefixes
    /// 3. PSR-0 prefixes
    pub fn resolve_class_file(&self, fqn: &str) -> Option<PathBuf> {
        let normalized = fqn.trim_start_matches('\\');

        // 1. Classmap lookup
        if let Some(path) = self.classmap.get(normalized) {
            return Some(path.clone());
        }

        // 2. PSR-4
        if let Some(path) = psr4::resolve_psr4(normalized, &self.psr4_prefixes) {
            return Some(path);
        }

        // 3. PSR-0
        if let Some(path) = psr4::resolve_psr0(normalized, &self.psr0_prefixes) {
            return Some(path);
        }

        None
    }

    /// Check if a file path is a project file (not in vendor/).
    pub fn is_project_file(&self, path: &Path) -> bool {
        if let Ok(rel) = path.strip_prefix(&self.project_root) {
            !rel.starts_with("vendor")
        } else {
            // Not under project root — consider it external
            false
        }
    }

    /// Discover all project PHP files using autoload configuration.
    ///
    /// Uses the PSR-4/PSR-0/classmap directories from `composer.json`
    /// to find PHP files in the project (excluding vendor).
    pub fn project_files(&self) -> Vec<PathBuf> {
        let source_dirs = discovery::project_source_dirs(
            Some(&AutoloadConfig {
                psr4: Some(
                    self.config
                        .psr4
                        .iter()
                        .map(|(k, v)| {
                            let relative: Vec<String> = v
                                .iter()
                                .filter_map(|p| {
                                    p.strip_prefix(&self.project_root)
                                        .ok()
                                        .map(|r| r.to_string_lossy().to_string())
                                })
                                .collect();
                            (
                                k.clone(),
                                if relative.len() == 1 {
                                    StringOrVec::Single(relative.into_iter().next().unwrap())
                                } else {
                                    StringOrVec::Multiple(relative)
                                },
                            )
                        })
                        .collect(),
                ),
                psr0: None,
                classmap: if self.config.classmap_globs.is_empty() {
                    None
                } else {
                    Some(self.config.classmap_globs.clone())
                },
                files: None,
            }),
            Some(&AutoloadConfig {
                psr4: Some(
                    self.config
                        .psr4_dev
                        .iter()
                        .map(|(k, v)| {
                            let relative: Vec<String> = v
                                .iter()
                                .filter_map(|p| {
                                    p.strip_prefix(&self.project_root)
                                        .ok()
                                        .map(|r| r.to_string_lossy().to_string())
                                })
                                .collect();
                            (
                                k.clone(),
                                if relative.len() == 1 {
                                    StringOrVec::Single(relative.into_iter().next().unwrap())
                                } else {
                                    StringOrVec::Multiple(relative)
                                },
                            )
                        })
                        .collect(),
                ),
                psr0: None,
                classmap: None,
                files: None,
            }),
            &self.project_root,
        );

        if source_dirs.is_empty() {
            // Fallback: if no autoload config, scan everything except vendor
            let all_dirs = vec![self.project_root.clone()];
            let mut files = discovery::discover_php_files(&all_dirs);
            files.retain(|f| self.is_project_file(f));
            files
        } else {
            discovery::discover_php_files(&source_dirs)
        }
    }

    /// Discover all vendor PHP files from installed package paths.
    pub fn vendor_files(&self) -> Vec<PathBuf> {
        let vendor_paths: Vec<PathBuf> = self
            .packages
            .iter()
            .map(|p| p.install_path.clone())
            .collect();
        let dirs = discovery::vendor_source_dirs(&vendor_paths);
        discovery::discover_php_files(&dirs)
    }

    /// All PHP files (project + vendor).
    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut all = self.project_files();
        all.extend(self.vendor_files());
        all.sort();
        all.dedup();
        all
    }

    /// Find which package a class FQN belongs to, using classmap + package paths.
    ///
    /// Resolution order:
    /// 1. Classmap lookup (exact FQN → file path → package)
    /// 2. PSR-4 prefix matching (FQN prefix → vendor directory → package)
    /// 3. Default: project package
    pub fn package_for_class(&self, fqn: &str) -> (&str, &str) {
        let normalized = fqn.trim_start_matches('\\');

        // Look up in classmap
        if let Some(file_path) = self.classmap.get(normalized) {
            return self.package_for_file(file_path);
        }

        // PSR-4: find matching prefix
        for (prefix, dirs) in &self.psr4_prefixes {
            if normalized.starts_with(prefix.as_str()) {
                for dir in dirs {
                    if dir.starts_with(self.project_root.join("vendor")) {
                        for pkg in &self.packages {
                            if dir.starts_with(&pkg.install_path) {
                                return (&pkg.name, &pkg.version);
                            }
                        }
                    }
                }
                break;
            }
        }

        // Default: project package
        (&self.config.name, &self.config.version)
    }

    /// Find the package that owns a given file path.
    ///
    /// Returns `(package_name, version)`. If no package matches,
    /// returns the project name and version.
    pub fn package_for_file(&self, path: &Path) -> (&str, &str) {
        for pkg in &self.packages {
            if path.starts_with(&pkg.install_path) {
                return (&pkg.name, &pkg.version);
            }
        }
        (&self.config.name, &self.config.version)
    }
}

/// Extract PSR-4 paths from an autoload config, resolving them to absolute paths.
fn extract_psr4_paths(
    autoload: Option<&AutoloadConfig>,
    project_root: &Path,
) -> HashMap<String, Vec<PathBuf>> {
    let mut result = HashMap::new();

    if let Some(config) = autoload {
        if let Some(psr4) = &config.psr4 {
            for (prefix, paths) in psr4 {
                let resolved: Vec<PathBuf> = paths
                    .to_vec()
                    .iter()
                    .map(|p| project_root.join(p.trim_end_matches('/')))
                    .collect();
                result.insert(prefix.clone(), resolved);
            }
        }
    }

    result
}

/// Load package information from composer.lock.
fn load_packages_from_lock(project_root: &Path) -> Vec<InstalledPackage> {
    let lock_path = project_root.join("composer.lock");
    if !lock_path.exists() {
        eprintln!(
            "scip-php-rust: warning: composer.lock not found at {:?}",
            lock_path
        );
        return Vec::new();
    }

    let content = match std::fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("scip-php-rust: warning: failed to read composer.lock: {}", e);
            return Vec::new();
        }
    };

    let lock: ComposerLock = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "scip-php-rust: warning: failed to parse composer.lock: {}",
                e
            );
            return Vec::new();
        }
    };

    let vendor_dir = project_root.join("vendor");

    let mut packages = Vec::new();
    for locked in lock.packages.iter().chain(lock.packages_dev.iter()) {
        let install_path = vendor_dir.join(&locked.name);
        packages.push(InstalledPackage {
            name: locked.name.clone(),
            version: locked.version.clone(),
            install_path,
            autoload: locked.autoload.clone().unwrap_or_default(),
        });
    }

    packages
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_minimal_project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // composer.json
        fs::write(
            root.join("composer.json"),
            r#"{
                "name": "test/project",
                "version": "1.0.0",
                "autoload": {
                    "psr-4": {
                        "App\\": "src/"
                    }
                },
                "autoload-dev": {
                    "psr-4": {
                        "Tests\\": "tests/"
                    }
                }
            }"#,
        )
        .unwrap();

        // Source files
        fs::create_dir_all(root.join("src/Models")).unwrap();
        fs::write(root.join("src/Models/User.php"), "<?php class User {}").unwrap();

        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("tests/UserTest.php"), "<?php class UserTest {}").unwrap();

        dir
    }

    fn setup_full_project() -> TempDir {
        let dir = setup_minimal_project();
        let root = dir.path();

        // composer.lock
        fs::write(
            root.join("composer.lock"),
            r#"{
                "packages": [
                    {
                        "name": "psr/log",
                        "version": "3.0.0",
                        "autoload": {
                            "psr-4": {
                                "Psr\\Log\\": "src"
                            }
                        }
                    }
                ],
                "packages-dev": []
            }"#,
        )
        .unwrap();

        // vendor directory with classmap
        fs::create_dir_all(root.join("vendor/composer")).unwrap();
        fs::create_dir_all(root.join("vendor/psr/log/src")).unwrap();
        fs::write(
            root.join("vendor/psr/log/src/LoggerInterface.php"),
            "<?php interface LoggerInterface {}",
        )
        .unwrap();

        // autoload_classmap.php
        fs::write(
            root.join("vendor/composer/autoload_classmap.php"),
            r#"<?php
$vendorDir = dirname(__DIR__);
$baseDir = dirname($vendorDir);
return array(
    'App\\Models\\User' => $baseDir . '/src/Models/User.php',
    'Psr\\Log\\LoggerInterface' => $vendorDir . '/psr/log/src/LoggerInterface.php',
);
"#,
        )
        .unwrap();

        // autoload_psr4.php
        fs::write(
            root.join("vendor/composer/autoload_psr4.php"),
            r#"<?php
$vendorDir = dirname(__DIR__);
$baseDir = dirname($vendorDir);
return array(
    'Psr\\Log\\' => array($vendorDir . '/psr/log/src'),
);
"#,
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_load_minimal_project() {
        let dir = setup_minimal_project();
        let composer = Composer::load(dir.path()).unwrap();

        assert_eq!(composer.config.name, "test/project");
        assert_eq!(composer.config.version, "1.0.0");
        assert!(composer.config.psr4.contains_key("App\\"));
        assert!(composer.config.psr4_dev.contains_key("Tests\\"));
    }

    #[test]
    fn test_load_no_composer_json() {
        let dir = tempfile::tempdir().unwrap();
        let composer = Composer::load(dir.path()).unwrap();

        assert_eq!(composer.config.name, "unknown");
        assert!(composer.config.psr4.is_empty());
    }

    #[test]
    fn test_load_vendor() {
        let dir = setup_full_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        composer.load_vendor().unwrap();

        // Classmap should be loaded
        assert!(composer.classmap.contains_key("App\\Models\\User"));
        assert!(composer.classmap.contains_key("Psr\\Log\\LoggerInterface"));

        // PSR-4 prefixes should include vendor prefixes
        let has_psr_log = composer
            .psr4_prefixes
            .iter()
            .any(|(p, _)| p == "Psr\\Log\\");
        assert!(has_psr_log, "PSR-4 should include Psr\\Log\\");
    }

    #[test]
    fn test_resolve_class_file_classmap() {
        let dir = setup_full_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        composer.load_vendor().unwrap();

        let result = composer.resolve_class_file("App\\Models\\User");
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("src/Models/User.php"));
    }

    #[test]
    fn test_resolve_class_file_psr4() {
        let dir = setup_full_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        composer.load_vendor().unwrap();

        // This class is in classmap AND resolvable via PSR-4
        let result = composer.resolve_class_file("Psr\\Log\\LoggerInterface");
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_class_file_not_found() {
        let dir = setup_full_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        composer.load_vendor().unwrap();

        let result = composer.resolve_class_file("NonExistent\\Class");
        assert!(result.is_none());
    }

    #[test]
    fn test_is_project_file() {
        let dir = setup_full_project();
        let composer = Composer::load(dir.path()).unwrap();

        assert!(composer.is_project_file(&dir.path().join("src/Models/User.php")));
        assert!(!composer.is_project_file(&dir.path().join("vendor/psr/log/src/LoggerInterface.php")));
    }

    #[test]
    fn test_package_for_file() {
        let dir = setup_full_project();
        let composer = Composer::load(dir.path()).unwrap();

        let (name, version) = composer.package_for_file(&dir.path().join("vendor/psr/log/src/LoggerInterface.php"));
        assert_eq!(name, "psr/log");
        assert_eq!(version, "3.0.0");

        let (name, _) = composer.package_for_file(&dir.path().join("src/Models/User.php"));
        assert_eq!(name, "test/project");
    }

    #[test]
    fn test_load_vendor_no_vendor_dir() {
        let dir = setup_minimal_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        // Should not error even without vendor/
        composer.load_vendor().unwrap();
        assert!(composer.classmap.is_empty());
    }

    #[test]
    fn test_project_files() {
        let dir = setup_minimal_project();
        let composer = Composer::load(dir.path()).unwrap();
        let files = composer.project_files();

        assert!(!files.is_empty());
        for f in &files {
            assert!(f.extension().map_or(false, |e| e == "php"));
        }
    }

    #[test]
    fn test_psr4_prefixes_sorted_longest_first() {
        let dir = setup_full_project();
        let mut composer = Composer::load(dir.path()).unwrap();
        composer.load_vendor().unwrap();

        // Verify prefixes are sorted longest first
        for i in 1..composer.psr4_prefixes.len() {
            assert!(
                composer.psr4_prefixes[i - 1].0.len() >= composer.psr4_prefixes[i].0.len(),
                "PSR-4 prefixes not sorted longest first: '{}' before '{}'",
                composer.psr4_prefixes[i - 1].0,
                composer.psr4_prefixes[i].0
            );
        }
    }

    #[test]
    fn test_extract_psr4_paths() {
        let root = Path::new("/project");
        let config = AutoloadConfig {
            psr4: Some(
                [("App\\".to_string(), StringOrVec::Single("src/".to_string()))]
                    .into_iter()
                    .collect(),
            ),
            psr0: None,
            classmap: None,
            files: None,
        };

        let result = extract_psr4_paths(Some(&config), root);
        assert_eq!(result.len(), 1);
        assert_eq!(result["App\\"], vec![PathBuf::from("/project/src")]);
    }

    #[test]
    fn test_extract_psr4_paths_empty() {
        let root = Path::new("/project");
        let result = extract_psr4_paths(None, root);
        assert!(result.is_empty());
    }
}
