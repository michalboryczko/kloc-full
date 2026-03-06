//! PhpStorm stubs index for identifying built-in PHP classes, functions, and constants.
//!
//! Loads the `PhpStormStubsMap.php` from the vendor directory if available,
//! otherwise operates with an empty index.

use std::collections::HashSet;
use std::path::Path;

use regex::Regex;

/// Index of PHP built-in symbols from JetBrains PhpStorm stubs.
#[derive(Debug)]
pub struct PhpStubIndex {
    classes: HashSet<String>,
    functions: HashSet<String>,
    constants: HashSet<String>,
}

impl PhpStubIndex {
    /// Create an empty index (no built-in symbols known).
    pub fn empty() -> Self {
        PhpStubIndex {
            classes: HashSet::new(),
            functions: HashSet::new(),
            constants: HashSet::new(),
        }
    }

    /// Load from PhpStormStubsMap.php if it exists at the given vendor path.
    ///
    /// Looks for `vendor/jetbrains/phpstorm-stubs/PhpStormStubsMap.php`.
    /// Returns an empty index if the file does not exist or cannot be parsed.
    pub fn load_from_vendor(vendor_dir: &Path) -> Self {
        let stubs_path = vendor_dir.join("jetbrains/phpstorm-stubs/PhpStormStubsMap.php");
        if !stubs_path.exists() {
            return Self::empty();
        }
        match std::fs::read_to_string(&stubs_path) {
            Ok(content) => Self::parse_stubs_map(&content),
            Err(_) => Self::empty(),
        }
    }

    /// Parse the PhpStormStubsMap.php file content.
    ///
    /// The file has sections like:
    /// ```php
    /// public const CLASSES = [
    ///     'ArrayAccess' => 'Core/Core.php',
    ///     'Iterator' => 'Core/Core.php',
    /// ];
    /// public const FUNCTIONS = [
    ///     'array_map' => 'standard/standard.php',
    /// ];
    /// public const CONSTANTS = [
    ///     'PHP_EOL' => 'standard/standard.php',
    /// ];
    /// ```
    fn parse_stubs_map(content: &str) -> Self {
        let re = Regex::new(r"'([^']+)'\s*=>\s*'([^']+)'").expect("invalid regex");

        let mut classes = HashSet::new();
        let mut functions = HashSet::new();
        let mut constants = HashSet::new();

        let mut section = "";
        for line in content.lines() {
            let trimmed = line.trim();

            // Detect section headers
            if (trimmed.contains("CLASSES") || trimmed.contains("$CLASSES"))
                && (trimmed.contains("= [") || trimmed.contains("= array("))
            {
                section = "classes";
            } else if (trimmed.contains("FUNCTIONS") || trimmed.contains("$FUNCTIONS"))
                && (trimmed.contains("= [") || trimmed.contains("= array("))
            {
                section = "functions";
            } else if (trimmed.contains("CONSTANTS") || trimmed.contains("$CONSTANTS"))
                && (trimmed.contains("= [") || trimmed.contains("= array("))
            {
                section = "constants";
            } else if trimmed == "];" || trimmed == ");" {
                // End of a section — but we just reset on next header
            } else if let Some(cap) = re.captures(line) {
                let name = cap[1].to_string();
                match section {
                    "classes" => {
                        classes.insert(name);
                    }
                    "functions" => {
                        functions.insert(name);
                    }
                    "constants" => {
                        constants.insert(name);
                    }
                    _ => {}
                }
            }
        }

        PhpStubIndex {
            classes,
            functions,
            constants,
        }
    }

    /// Check if a class name is a built-in PHP class.
    ///
    /// Strips a leading backslash if present.
    pub fn is_builtin_class(&self, name: &str) -> bool {
        self.classes.contains(name.trim_start_matches('\\'))
    }

    /// Check if a function name is a built-in PHP function.
    pub fn is_builtin_function(&self, name: &str) -> bool {
        self.functions.contains(name.trim_start_matches('\\'))
    }

    /// Check if a constant name is a built-in PHP constant.
    pub fn is_builtin_constant(&self, name: &str) -> bool {
        self.constants.contains(name.trim_start_matches('\\'))
    }

    /// Number of known built-in classes.
    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    /// Number of known built-in functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Number of known built-in constants.
    pub fn constant_count(&self) -> usize {
        self.constants.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STUBS_MAP: &str = r#"<?php

namespace StubsMap;

class PhpStormStubsMap
{
    public const CLASSES = [
        'ArrayAccess' => 'Core/Core.php',
        'Iterator' => 'Core/Core.php',
        'PDO' => 'PDO/PDO.php',
        'DateTime' => 'date/date.php',
    ];

    public const FUNCTIONS = [
        'array_map' => 'standard/standard.php',
        'strlen' => 'standard/standard.php',
        'json_encode' => 'json/json.php',
    ];

    public const CONSTANTS = [
        'PHP_EOL' => 'standard/standard.php',
        'PHP_INT_MAX' => 'standard/standard.php',
        'SORT_ASC' => 'standard/standard.php',
    ];
}
"#;

    #[test]
    fn test_parse_stubs_map() {
        let index = PhpStubIndex::parse_stubs_map(SAMPLE_STUBS_MAP);

        assert_eq!(index.class_count(), 4);
        assert_eq!(index.function_count(), 3);
        assert_eq!(index.constant_count(), 3);
    }

    #[test]
    fn test_is_builtin_class() {
        let index = PhpStubIndex::parse_stubs_map(SAMPLE_STUBS_MAP);

        assert!(index.is_builtin_class("ArrayAccess"));
        assert!(index.is_builtin_class("PDO"));
        assert!(index.is_builtin_class("DateTime"));
        assert!(!index.is_builtin_class("MyCustomClass"));
    }

    #[test]
    fn test_is_builtin_class_with_leading_backslash() {
        let index = PhpStubIndex::parse_stubs_map(SAMPLE_STUBS_MAP);

        assert!(index.is_builtin_class("\\ArrayAccess"));
        assert!(index.is_builtin_class("\\PDO"));
    }

    #[test]
    fn test_is_builtin_function() {
        let index = PhpStubIndex::parse_stubs_map(SAMPLE_STUBS_MAP);

        assert!(index.is_builtin_function("array_map"));
        assert!(index.is_builtin_function("strlen"));
        assert!(index.is_builtin_function("json_encode"));
        assert!(!index.is_builtin_function("my_custom_function"));
    }

    #[test]
    fn test_is_builtin_constant() {
        let index = PhpStubIndex::parse_stubs_map(SAMPLE_STUBS_MAP);

        assert!(index.is_builtin_constant("PHP_EOL"));
        assert!(index.is_builtin_constant("PHP_INT_MAX"));
        assert!(!index.is_builtin_constant("MY_CONSTANT"));
    }

    #[test]
    fn test_empty_index() {
        let index = PhpStubIndex::empty();

        assert_eq!(index.class_count(), 0);
        assert_eq!(index.function_count(), 0);
        assert_eq!(index.constant_count(), 0);
        assert!(!index.is_builtin_class("PDO"));
    }

    #[test]
    fn test_load_from_vendor_missing() {
        let dir = tempfile::tempdir().unwrap();
        let index = PhpStubIndex::load_from_vendor(dir.path());
        assert_eq!(index.class_count(), 0);
    }

    #[test]
    fn test_load_from_vendor_exists() {
        let dir = tempfile::tempdir().unwrap();
        let stubs_dir = dir.path().join("jetbrains/phpstorm-stubs");
        std::fs::create_dir_all(&stubs_dir).unwrap();
        std::fs::write(stubs_dir.join("PhpStormStubsMap.php"), SAMPLE_STUBS_MAP).unwrap();

        let index = PhpStubIndex::load_from_vendor(dir.path());
        assert_eq!(index.class_count(), 4);
        assert_eq!(index.function_count(), 3);
    }
}
