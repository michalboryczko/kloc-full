//! Serde deserialization structs for composer.json and composer.lock.

use serde::Deserialize;
use std::collections::HashMap;

/// Root structure of a `composer.json` file.
#[derive(Debug, Deserialize)]
pub struct ComposerJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub autoload: Option<AutoloadConfig>,
    #[serde(rename = "autoload-dev")]
    pub autoload_dev: Option<AutoloadConfig>,
    pub require: Option<HashMap<String, String>>,
    #[serde(rename = "require-dev")]
    pub require_dev: Option<HashMap<String, String>>,
}

/// Autoload configuration section (PSR-4, PSR-0, classmap, files).
#[derive(Debug, Deserialize, Default, Clone)]
pub struct AutoloadConfig {
    #[serde(rename = "psr-4", default)]
    pub psr4: Option<HashMap<String, StringOrVec>>,
    #[serde(rename = "psr-0", default)]
    pub psr0: Option<HashMap<String, StringOrVec>>,
    #[serde(default)]
    pub classmap: Option<Vec<String>>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
}

/// A value that can be either a single string or a list of strings.
/// Composer autoload paths can be specified as `"src/"` or `["src/", "lib/"]`.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    /// Normalize to a Vec of string slices.
    pub fn to_vec(&self) -> Vec<&str> {
        match self {
            StringOrVec::Single(s) => vec![s.as_str()],
            StringOrVec::Multiple(v) => v.iter().map(String::as_str).collect(),
        }
    }

    /// Convert to owned Strings.
    pub fn into_strings(self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s],
            StringOrVec::Multiple(v) => v,
        }
    }
}

/// Root structure of a `composer.lock` file.
#[derive(Debug, Deserialize)]
pub struct ComposerLock {
    #[serde(default)]
    pub packages: Vec<LockedPackage>,
    #[serde(rename = "packages-dev", default)]
    pub packages_dev: Vec<LockedPackage>,
}

/// A locked package entry from `composer.lock`.
#[derive(Debug, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub autoload: Option<AutoloadConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composer_json_minimal() {
        let json = r#"{"name": "test/project", "require": {"php": ">=8.0"}}"#;
        let parsed: ComposerJson = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("test/project"));
        assert!(parsed.autoload.is_none());
    }

    #[test]
    fn test_parse_composer_json_with_autoload() {
        let json = r#"{
            "name": "test/project",
            "autoload": {
                "psr-4": {
                    "App\\": "src/",
                    "Database\\": ["database/seeds/", "database/factories/"]
                },
                "classmap": ["legacy/"],
                "files": ["helpers.php"]
            },
            "autoload-dev": {
                "psr-4": {
                    "Tests\\": "tests/"
                }
            }
        }"#;
        let parsed: ComposerJson = serde_json::from_str(json).unwrap();

        let autoload = parsed.autoload.unwrap();
        let psr4 = autoload.psr4.unwrap();
        assert_eq!(psr4.len(), 2);
        assert_eq!(psr4["App\\"].to_vec(), vec!["src/"]);
        assert_eq!(
            psr4["Database\\"].to_vec(),
            vec!["database/seeds/", "database/factories/"]
        );
        assert_eq!(autoload.classmap.unwrap(), vec!["legacy/"]);
        assert_eq!(autoload.files.unwrap(), vec!["helpers.php"]);

        let autoload_dev = parsed.autoload_dev.unwrap();
        let psr4_dev = autoload_dev.psr4.unwrap();
        assert_eq!(psr4_dev["Tests\\"].to_vec(), vec!["tests/"]);
    }

    #[test]
    fn test_parse_composer_lock() {
        let json = r#"{
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
            "packages-dev": [
                {
                    "name": "phpunit/phpunit",
                    "version": "10.0.0",
                    "autoload": {
                        "classmap": ["src/"]
                    }
                }
            ]
        }"#;
        let parsed: ComposerLock = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].name, "psr/log");
        assert_eq!(parsed.packages[0].version, "3.0.0");
        assert_eq!(parsed.packages_dev.len(), 1);
        assert_eq!(parsed.packages_dev[0].name, "phpunit/phpunit");
    }

    #[test]
    fn test_string_or_vec_single() {
        let val: StringOrVec = serde_json::from_str(r#""src/""#).unwrap();
        assert_eq!(val.to_vec(), vec!["src/"]);
        assert_eq!(val.into_strings(), vec!["src/".to_string()]);
    }

    #[test]
    fn test_string_or_vec_multiple() {
        let val: StringOrVec = serde_json::from_str(r#"["src/", "lib/"]"#).unwrap();
        assert_eq!(val.to_vec(), vec!["src/", "lib/"]);
    }

    #[test]
    fn test_parse_empty_autoload() {
        let json = r#"{"name": "test/project", "autoload": {}}"#;
        let parsed: ComposerJson = serde_json::from_str(json).unwrap();
        let autoload = parsed.autoload.unwrap();
        assert!(autoload.psr4.is_none());
        assert!(autoload.psr0.is_none());
        assert!(autoload.classmap.is_none());
        assert!(autoload.files.is_none());
    }

    #[test]
    fn test_composer_lock_empty_packages() {
        let json = r#"{"packages": [], "packages-dev": []}"#;
        let parsed: ComposerLock = serde_json::from_str(json).unwrap();
        assert!(parsed.packages.is_empty());
        assert!(parsed.packages_dev.is_empty());
    }
}
