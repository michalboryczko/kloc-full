//! CST parsing via tree-sitter-php and AST adapter layer.

pub mod ast;
pub mod cst;
pub mod position;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Wraps tree-sitter Parser pre-configured for PHP.
///
/// NOT thread-safe — create one per thread (or use thread_local!).
/// The Language is shared; only Parser itself is per-thread.
///
/// # Example usage with rayon (in indexing phase):
///
/// ```ignore
/// files.par_iter().for_each(|file| {
///     thread_local! {
///         static PARSER: std::cell::RefCell<PhpParser> =
///             std::cell::RefCell::new(PhpParser::new());
///     }
///
///     PARSER.with(|parser| {
///         let mut parser = parser.borrow_mut();
///         let parsed = parser.parse_file(file).expect("parse failed");
///         // ... use parsed
///     });
/// });
/// ```
pub struct PhpParser {
    parser: tree_sitter::Parser,
}

impl PhpParser {
    /// Create a new parser configured for PHP.
    ///
    /// This is cheap (~1 microsecond) — safe to call in every rayon task.
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .expect("Failed to load tree-sitter-php language");
        PhpParser { parser }
    }

    /// Parse PHP source text and return a ParsedFile.
    ///
    /// # Errors
    /// Returns error if tree-sitter fails to produce a tree (extremely rare).
    /// Note: PHP syntax errors do NOT return Err — they result in error nodes in the tree.
    pub fn parse(&mut self, source: &str, path: impl Into<PathBuf>) -> Result<ParsedFile> {
        let source_bytes = source.as_bytes().to_vec();
        let tree = self
            .parser
            .parse(&source_bytes, None)
            .context("tree-sitter failed to produce a parse tree")?;

        Ok(ParsedFile {
            tree,
            source: source_bytes,
            path: path.into(),
        })
    }

    /// Parse a PHP file from disk.
    pub fn parse_file(&mut self, path: &Path) -> Result<ParsedFile> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read PHP file: {:?}", path))?;
        self.parse(&source, path.to_owned())
    }
}

/// The result of parsing a PHP file.
pub struct ParsedFile {
    /// The tree-sitter parse tree.
    pub tree: tree_sitter::Tree,
    /// Source bytes (UTF-8).
    pub source: Vec<u8>,
    /// File path (for error reporting and SCIP output).
    pub path: PathBuf,
}

impl ParsedFile {
    /// Get the root CST node.
    pub fn root(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }

    /// Check if the parse had errors.
    pub fn has_errors(&self) -> bool {
        self.tree.root_node().has_error()
    }

    /// Get source text as str.
    pub fn source_str(&self) -> &str {
        // SAFETY: we constructed source from a valid &str in parse()
        unsafe { std::str::from_utf8_unchecked(&self.source) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_php() {
        let mut parser = PhpParser::new();
        let result = parser.parse("<?php\necho 'hello';", "test.php");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(!parsed.has_errors());
        assert_eq!(parsed.root().kind(), "program");
    }

    #[test]
    fn test_parse_with_syntax_error() {
        let mut parser = PhpParser::new();
        let result = parser.parse("<?php\nfunction {}", "bad.php");
        assert!(result.is_ok()); // parse still succeeds
        let parsed = result.unwrap();
        assert!(parsed.has_errors()); // but tree has error nodes
    }

    #[test]
    fn test_parse_empty_file() {
        let mut parser = PhpParser::new();
        let result = parser.parse("", "empty.php");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_php8_features() {
        let mut parser = PhpParser::new();
        let php8 = r#"<?php
        enum Status: string {
            case Active = 'active';
            case Inactive = 'inactive';
        }

        function process(int|string $value): void {}

        $result = match($value) {
            1 => 'one',
            default => 'other',
        };
        "#;
        let result = parser.parse(php8, "php8.php");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(!parsed.has_errors(), "PHP 8 features should parse without errors");
    }

    #[test]
    fn test_source_str_roundtrip() {
        let mut parser = PhpParser::new();
        let source = "<?php\nclass Foo {}";
        let parsed = parser.parse(source, "test.php").unwrap();
        assert_eq!(parsed.source_str(), source);
    }
}
