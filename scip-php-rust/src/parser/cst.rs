//! Utility functions for tree-sitter CST node traversal.

use tree_sitter::Node;

/// Extract the UTF-8 text content of a node from the source bytes.
///
/// # Example
/// ```ignore
/// // For node spanning bytes 5..10 in source "<?php class Foo {}"
/// // node_text(node, source) returns "class"
/// ```
#[inline]
pub fn node_text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    let bytes = &source[node.start_byte()..node.end_byte()];
    // SAFETY: tree-sitter guarantees byte offsets are valid UTF-8 boundaries
    // for PHP source (which is required to be valid UTF-8 by PHP itself).
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

/// Find the first NAMED child of a node with the given CST kind.
///
/// Uses `node.named_child_count()` / `node.named_child()` to skip anonymous nodes
/// (punctuation, keywords represented as literals in the grammar).
pub fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.named_child_count() {
        let child = node.named_child(i)?;
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

/// Find ALL named children of a node with the given CST kind.
pub fn children_by_kind<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == kind {
                result.push(child);
            }
        }
    }
    result
}

/// Get all named children of a node (excludes anonymous nodes).
pub fn named_children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut result = Vec::with_capacity(node.named_child_count());
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            result.push(child);
        }
    }
    result
}

/// Get all children of a node, including anonymous ones (punctuation, keywords).
pub fn all_children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut result = Vec::with_capacity(node.child_count());
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            result.push(child);
        }
    }
    result
}

/// Walk up the parent chain looking for a node of the given kind.
/// Returns the ancestor if found, None if we reach the root.
pub fn find_ancestor<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = node.parent()?;
    loop {
        if current.kind() == kind {
            return Some(current);
        }
        current = current.parent()?;
    }
}

/// Walk up the parent chain looking for any node matching the predicate.
pub fn find_ancestor_where<'a, F>(node: Node<'a>, predicate: F) -> Option<Node<'a>>
where
    F: Fn(Node<'a>) -> bool,
{
    let mut current = node.parent()?;
    loop {
        if predicate(current) {
            return Some(current);
        }
        current = current.parent()?;
    }
}

/// Check if a node has an ancestor of the given kind.
#[inline]
pub fn has_ancestor(node: Node<'_>, kind: &str) -> bool {
    find_ancestor(node, kind).is_some()
}

/// Check if a node has a child (named or anonymous) with the given text.
/// Useful for detecting keywords like "static", "readonly", "abstract".
pub fn has_child_text(node: Node<'_>, text: &str, source: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if node_text(child, source) == text {
                return true;
            }
        }
    }
    false
}

/// Check if a named child exists with the given kind.
#[inline]
pub fn has_named_child(node: Node<'_>, kind: &str) -> bool {
    child_by_kind(node, kind).is_some()
}

/// Recursively find all nodes of the given kind in the subtree.
/// Results are returned in pre-order (parent before children).
pub fn find_all<'a>(node: Node<'a>, kind: &str, results: &mut Vec<Node<'a>>) {
    if node.kind() == kind && node.is_named() {
        results.push(node);
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            find_all(child, kind, results);
        }
    }
}

/// Get the first non-comment, non-whitespace child of a node.
/// Useful for getting the "real" first token.
pub fn first_significant_child<'a>(node: Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() != "comment" {
                return Some(child);
            }
        }
    }
    None
}

/// Get the preceding sibling that is a doc comment (`/** ... */`).
/// Used to extract PHPDoc for class/method/property declarations.
pub fn preceding_doc_comment<'a>(node: Node<'a>, source: &'a [u8]) -> Option<&'a str> {
    let mut sibling = node.prev_named_sibling()?;
    // Walk backwards through preceding siblings looking for a doc comment
    loop {
        if sibling.kind() == "comment" {
            let text = node_text(sibling, source);
            if text.starts_with("/**") {
                return Some(text);
            }
            // Not a doc comment; stop looking
            return None;
        }
        sibling = sibling.prev_named_sibling()?;
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
    fn test_node_text() {
        let (tree, source) = parse_php("<?php class Foo {}");
        let root = tree.root_node();
        // Find class name node
        let class_decl = child_by_kind(root, "class_declaration").unwrap();
        let name = class_decl.child_by_field_name("name").unwrap();
        assert_eq!(node_text(name, &source), "Foo");
    }

    #[test]
    fn test_child_by_kind_found() {
        let (tree, _) = parse_php("<?php class Foo {}");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration");
        assert!(class.is_some());
        assert_eq!(class.unwrap().kind(), "class_declaration");
    }

    #[test]
    fn test_child_by_kind_not_found() {
        let (tree, _) = parse_php("<?php class Foo {}");
        let root = tree.root_node();
        let func = child_by_kind(root, "function_definition");
        assert!(func.is_none());
    }

    #[test]
    fn test_children_by_kind_multiple() {
        let (tree, _source) = parse_php("<?php class Foo { public $a; public $b; }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let props = children_by_kind(body, "property_declaration");
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn test_find_ancestor() {
        let (tree, _) = parse_php("<?php class Foo { public function bar() {} }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let method = child_by_kind(body, "method_declaration").unwrap();
        let method_name = method.child_by_field_name("name").unwrap();

        let ancestor = find_ancestor(method_name, "class_declaration");
        assert!(ancestor.is_some());
        assert_eq!(ancestor.unwrap().kind(), "class_declaration");
    }

    #[test]
    fn test_find_ancestor_not_found() {
        let (tree, _) = parse_php("<?php class Foo {}");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        // No namespace ancestor
        let ns = find_ancestor(class, "namespace_definition");
        assert!(ns.is_none());
    }

    #[test]
    fn test_has_child_text_static() {
        let (tree, source) = parse_php("<?php class Foo { static public $bar; }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let prop = child_by_kind(body, "property_declaration").unwrap();
        assert!(has_child_text(prop, "static", &source));
        assert!(!has_child_text(prop, "readonly", &source));
    }

    #[test]
    fn test_preceding_doc_comment() {
        let (tree, source) = parse_php(
            "<?php\n/** This is a doc comment */\nclass Foo {}",
        );
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let doc = preceding_doc_comment(class, &source);
        assert!(doc.is_some());
        assert!(doc.unwrap().contains("This is a doc comment"));
    }
}
