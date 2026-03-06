//! File-level name resolution traversal.
//!
//! `FileNameResolver` wraps `NameResolver` and provides a traversal-based API
//! for initializing from a parsed PHP file and tracking class context as nodes
//! are entered/exited during tree walking.

use tree_sitter::Node;

use crate::parser::cst::{child_by_kind, find_all, node_text};

use super::resolver::{ClassResolution, NameContext, NameResolution, NameResolver};

/// A file-scoped name resolver that initializes from a CST root and tracks
/// class context during traversal.
pub struct FileNameResolver {
    resolver: NameResolver,
}

impl FileNameResolver {
    /// Create a new file-level name resolver.
    pub fn new() -> Self {
        FileNameResolver {
            resolver: NameResolver::new(),
        }
    }

    /// Initialize the resolver by scanning the file's top-level nodes for
    /// `namespace_definition` and `namespace_use_declaration` nodes.
    ///
    /// This sets up the namespace and all imports before any traversal begins.
    pub fn initialize_from_file(&mut self, root: Node, source: &[u8]) {
        // Find and process namespace definitions
        let mut ns_nodes = Vec::new();
        find_all(root, "namespace_definition", &mut ns_nodes);
        for ns_node in &ns_nodes {
            if let Some(ns_name) = child_by_kind(*ns_node, "namespace_name") {
                self.resolver.enter_namespace(node_text(ns_name, source));
            }
        }

        // Find and process use declarations
        let mut use_nodes = Vec::new();
        find_all(root, "namespace_use_declaration", &mut use_nodes);
        for use_node in &use_nodes {
            let imports = NameResolver::parse_use_declaration(*use_node, source);
            for import in imports {
                self.resolver.add_import(import);
            }
        }
    }

    /// Handle entering a CST node during traversal.
    ///
    /// Returns `true` if this node was handled (a scope-defining node),
    /// `false` otherwise.
    ///
    /// Handles:
    /// - `namespace_definition`: updates namespace
    /// - `class_declaration`, `interface_declaration`, `trait_declaration`,
    ///   `enum_declaration`: pushes class context with resolved parent
    pub fn on_enter_node(&mut self, node: Node, source: &[u8]) -> bool {
        match node.kind() {
            "namespace_definition" => {
                if let Some(ns_name) = child_by_kind(node, "namespace_name") {
                    self.resolver.enter_namespace(node_text(ns_name, source));
                }
                true
            }
            "class_declaration" | "interface_declaration" | "trait_declaration"
            | "enum_declaration" => {
                let class_name = node
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");

                // Build FQN
                let fqn = if self.resolver.namespace().is_empty() {
                    class_name.to_string()
                } else {
                    format!("{}\\{}", self.resolver.namespace(), class_name)
                };

                // Resolve parent class (from base_clause for class_declaration)
                let parent_fqn = self.resolve_parent_class(node, source);

                self.resolver.push_class(&fqn, parent_fqn.as_deref());
                true
            }
            _ => false,
        }
    }

    /// Handle exiting a CST node during traversal.
    ///
    /// Pops class context when leaving class-like declarations.
    pub fn on_exit_node(&mut self, node: Node) {
        match node.kind() {
            "class_declaration" | "interface_declaration" | "trait_declaration"
            | "enum_declaration" => {
                self.resolver.pop_class();
            }
            _ => {}
        }
    }

    /// Resolve a name in the given context.
    pub fn resolve_name(&self, name: &str, context: NameContext) -> NameResolution {
        self.resolver.resolve_name(name, context)
    }

    /// Resolve a class name to its FQN.
    pub fn resolve_class(&self, name: &str) -> String {
        self.resolver.resolve_class(name)
    }

    /// Resolve a class name handling special names (self, static, parent, built-ins).
    pub fn resolve_class_or_special(&self, name: &str) -> ClassResolution {
        self.resolver.resolve_class_or_special(name)
    }

    /// Get a reference to the underlying resolver.
    pub fn resolver(&self) -> &NameResolver {
        &self.resolver
    }

    /// Get the current namespace.
    pub fn namespace(&self) -> &str {
        self.resolver.namespace()
    }

    // ─── Private helpers ───────────────────────────────────────────────────

    /// Extract and resolve the parent class from a `base_clause` in a class declaration.
    fn resolve_parent_class(&self, node: Node, source: &[u8]) -> Option<String> {
        // Only class_declaration has base_clause (extends)
        // Note: base_clause is a node kind, not a field name
        let base_clause = child_by_kind(node, "base_clause")?;

        // The parent name is a name or qualified_name child of base_clause
        let parent_name_node = child_by_kind(base_clause, "qualified_name")
            .or_else(|| child_by_kind(base_clause, "name"))?;

        let parent_name = node_text(parent_name_node, source);
        Some(self.resolver.resolve_class(parent_name))
    }
}

impl Default for FileNameResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_php(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let source_bytes = source.as_bytes().to_vec();
        let tree = parser.parse(&source_bytes, None).unwrap();
        (tree, source_bytes)
    }

    #[test]
    fn test_file_resolver_basic() {
        let source = r#"<?php

namespace App\Services;

use App\Models\User;
use Psr\Log\LoggerInterface as Logger;

class UserService {}
"#;
        let (tree, source_bytes) = parse_php(source);
        let mut resolver = FileNameResolver::new();
        resolver.initialize_from_file(tree.root_node(), &source_bytes);

        assert_eq!(resolver.namespace(), "App\\Services");
        assert_eq!(resolver.resolve_class("User"), "App\\Models\\User");
        assert_eq!(
            resolver.resolve_class("Logger"),
            "Psr\\Log\\LoggerInterface"
        );
        assert_eq!(
            resolver.resolve_class("UserService"),
            "App\\Services\\UserService"
        );
    }

    #[test]
    fn test_file_resolver_with_class_traversal() {
        let source = r#"<?php

namespace App;

use App\Base\BaseService;

class MyService extends BaseService {}
"#;
        let (tree, source_bytes) = parse_php(source);
        let root = tree.root_node();
        let mut resolver = FileNameResolver::new();
        resolver.initialize_from_file(root, &source_bytes);

        // Find and enter the class declaration
        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "class_declaration" {
                    resolver.on_enter_node(child, &source_bytes);
                    assert_eq!(
                        resolver.resolver().current_class_fqn(),
                        Some("App\\MyService")
                    );
                    assert_eq!(
                        resolver.resolver().current_parent_fqn(),
                        Some("App\\Base\\BaseService")
                    );

                    // self/parent should resolve
                    let self_res = resolver.resolve_class_or_special("self");
                    assert_eq!(
                        self_res,
                        ClassResolution::Resolved("App\\MyService".to_string())
                    );
                    let parent_res = resolver.resolve_class_or_special("parent");
                    assert_eq!(
                        parent_res,
                        ClassResolution::Resolved("App\\Base\\BaseService".to_string())
                    );

                    resolver.on_exit_node(child);
                    assert!(resolver.resolver().current_class_fqn().is_none());
                }
            }
        }
    }

    #[test]
    fn test_file_resolver_interface() {
        let source = r#"<?php

namespace App\Contracts;

interface Cacheable {}
"#;
        let (tree, source_bytes) = parse_php(source);
        let root = tree.root_node();
        let mut resolver = FileNameResolver::new();
        resolver.initialize_from_file(root, &source_bytes);

        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "interface_declaration" {
                    resolver.on_enter_node(child, &source_bytes);
                    assert_eq!(
                        resolver.resolver().current_class_fqn(),
                        Some("App\\Contracts\\Cacheable")
                    );
                    resolver.on_exit_node(child);
                }
            }
        }
    }

    #[test]
    fn test_file_resolver_global_namespace() {
        let source = "<?php\nclass Foo {}\n";
        let (tree, source_bytes) = parse_php(source);
        let root = tree.root_node();
        let mut resolver = FileNameResolver::new();
        resolver.initialize_from_file(root, &source_bytes);

        assert_eq!(resolver.namespace(), "");
        assert_eq!(resolver.resolve_class("Foo"), "Foo");

        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "class_declaration" {
                    resolver.on_enter_node(child, &source_bytes);
                    assert_eq!(resolver.resolver().current_class_fqn(), Some("Foo"));
                    resolver.on_exit_node(child);
                }
            }
        }
    }
}
