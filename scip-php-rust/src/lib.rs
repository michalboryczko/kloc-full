pub mod parser;
pub mod names;
pub mod types;
pub mod indexing;
pub mod composer;
pub mod output;
pub mod symbol;
pub mod discovery;
pub mod pipeline;

/// Verify that tree-sitter-php is linked correctly.
///
/// ```
/// let mut parser = tree_sitter::Parser::new();
/// parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into()).unwrap();
/// let tree = parser.parse("<?php echo 'hello';", None).unwrap();
/// assert!(!tree.root_node().has_error());
/// ```
pub fn parser_works() {}
