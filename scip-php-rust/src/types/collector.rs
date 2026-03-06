//! Type collection from PHP CST (Pass 1).
//!
//! Walks a parsed PHP file's CST to extract type definitions (classes, interfaces,
//! traits, enums, functions) and their members (methods, properties) into the
//! global `TypeDatabase`.

use std::path::Path;

use tree_sitter::{Node, Tree};

use crate::names::NameResolver;
use crate::parser::cst::{child_by_kind, children_by_kind, node_text, preceding_doc_comment};

use super::{ParamInfo, SymbolKind, TypeDatabase, TypeDef, Visibility};

// ═══════════════════════════════════════════════════════════════════════════════
// Public entry point
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect type definitions from a single parsed PHP file into the database.
///
/// Creates a fresh `NameResolver` for the file and walks the CST to extract
/// all classes, interfaces, traits, enums, and standalone functions.
pub fn collect_defs_from_file(
    file_path: &Path,
    source: &[u8],
    tree: &Tree,
    db: &TypeDatabase,
) {
    let root = tree.root_node();
    let mut resolver = NameResolver::new();

    collect_from_node(root, source, &mut resolver, db, file_path);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Recursive dispatcher
// ═══════════════════════════════════════════════════════════════════════════════

/// Recursively walk CST nodes and dispatch to the appropriate collector.
fn collect_from_node(
    node: Node,
    source: &[u8],
    resolver: &mut NameResolver,
    db: &TypeDatabase,
    file_path: &Path,
) {
    match node.kind() {
        "program" | "declaration_list" | "enum_declaration_list" => {
            // Recurse into children
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    collect_from_node(child, source, resolver, db, file_path);
                }
            }
        }
        "namespace_definition" => {
            // Extract namespace name
            if let Some(ns_name) = child_by_kind(node, "namespace_name") {
                resolver.enter_namespace(node_text(ns_name, source));
            }

            // Braced namespace: recurse into compound_statement body
            if let Some(body) = child_by_kind(node, "compound_statement") {
                for i in 0..body.named_child_count() {
                    if let Some(child) = body.named_child(i) {
                        collect_from_node(child, source, resolver, db, file_path);
                    }
                }
            }
            // Semicolon namespace: siblings are handled by the parent loop
        }
        "namespace_use_declaration" => {
            // Parse use statements and add to resolver
            let imports = NameResolver::parse_use_declaration(node, source);
            for import in imports {
                resolver.add_import(import);
            }
        }
        "class_declaration" => {
            collect_class_def(node, source, resolver, db, file_path, SymbolKind::Class);
        }
        "interface_declaration" => {
            collect_class_def(
                node,
                source,
                resolver,
                db,
                file_path,
                SymbolKind::Interface,
            );
        }
        "trait_declaration" => {
            collect_class_def(node, source, resolver, db, file_path, SymbolKind::Trait);
        }
        "enum_declaration" => {
            collect_enum_def(node, source, resolver, db, file_path);
        }
        "function_definition" => {
            collect_function_def(node, source, resolver, db, file_path);
        }
        _ => {
            // Unknown node kind — skip
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Class / Interface / Trait collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect a class, interface, or trait definition from a CST node.
fn collect_class_def(
    node: Node,
    source: &[u8],
    resolver: &mut NameResolver,
    db: &TypeDatabase,
    file_path: &Path,
    base_kind: SymbolKind,
) {
    // Extract name
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, source),
        None => return, // anonymous class — skip
    };

    // Build FQN
    let fqn = qualify_definition(resolver, name);

    // Determine modifiers
    let modifiers = collect_modifiers(node, source);
    let is_abstract = modifiers.contains(&"abstract");
    let is_final = modifiers.contains(&"final");
    let is_readonly = modifiers.contains(&"readonly");

    // Determine the actual kind based on modifiers
    let kind = if base_kind == SymbolKind::Class {
        if is_abstract {
            SymbolKind::AbstractClass
        } else if is_final {
            SymbolKind::FinalClass
        } else {
            SymbolKind::Class
        }
    } else {
        base_kind
    };

    // Extract docblock
    let docblock = preceding_doc_comment(node, source).map(|s| s.to_string());

    // Insert the type definition
    let def = TypeDef {
        kind,
        file_path: file_path.to_owned(),
        is_abstract,
        is_final,
        is_readonly,
        enum_backing_type: None,
        docblock,
    };
    db.insert_def(&fqn, def);

    // Collect uppers (extends + implements)
    let mut uppers = Vec::new();

    // base_clause = extends (for classes)
    if let Some(base_clause) = child_by_kind(node, "base_clause") {
        collect_parent_names(base_clause, source, resolver, &mut uppers);
    }

    // class_interface_clause = implements (for classes) or extends (for interfaces)
    if let Some(iface_clause) = child_by_kind(node, "class_interface_clause") {
        collect_parent_names(iface_clause, source, resolver, &mut uppers);
    }

    if !uppers.is_empty() {
        db.add_uppers(&fqn, uppers.clone());
    }

    // Push class context for method resolution of self/parent
    let parent_fqn = uppers.first().cloned();
    resolver.push_class(&fqn, parent_fqn.as_deref());

    // Collect body members
    if let Some(body) = node.child_by_field_name("body") {
        collect_class_body(body, source, resolver, db, &fqn);
    }

    resolver.pop_class();
}

/// Collect an enum definition.
fn collect_enum_def(
    node: Node,
    source: &[u8],
    resolver: &mut NameResolver,
    db: &TypeDatabase,
    file_path: &Path,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, source),
        None => return,
    };

    let fqn = qualify_definition(resolver, name);

    // Detect backing type (e.g. `: string` or `: int`)
    // The backing type is a child node after the `:` token
    let enum_backing_type = extract_enum_backing_type(node, source);

    let docblock = preceding_doc_comment(node, source).map(|s| s.to_string());

    let def = TypeDef {
        kind: SymbolKind::Enum,
        file_path: file_path.to_owned(),
        is_abstract: false,
        is_final: false,
        is_readonly: false,
        enum_backing_type,
        docblock,
    };
    db.insert_def(&fqn, def);

    // Enums can implement interfaces
    let mut uppers = Vec::new();
    if let Some(iface_clause) = child_by_kind(node, "class_interface_clause") {
        collect_parent_names(iface_clause, source, resolver, &mut uppers);
    }
    if !uppers.is_empty() {
        db.add_uppers(&fqn, uppers);
    }

    resolver.push_class(&fqn, None);

    // Collect body — enum uses `enum_declaration_list` instead of `declaration_list`
    if let Some(body) = child_by_kind(node, "enum_declaration_list") {
        collect_class_body(body, source, resolver, db, &fqn);
    }

    resolver.pop_class();
}

/// Extract the enum backing type from an enum_declaration node.
/// In tree-sitter-php, the backing type is a primitive_type or named_type child
/// after a `:` token.
fn extract_enum_backing_type(node: Node, source: &[u8]) -> Option<String> {
    let mut found_colon = false;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() && node_text(child, source) == ":" {
                found_colon = true;
                continue;
            }
            if found_colon && child.is_named() {
                match child.kind() {
                    "primitive_type" | "named_type" | "name" => {
                        return Some(node_text(child, source).to_string());
                    }
                    "enum_declaration_list" | "class_interface_clause" => {
                        // We've gone past the backing type into the body or interfaces
                        return None;
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Collect parent/interface names from a base_clause or class_interface_clause.
fn collect_parent_names(
    clause: Node,
    source: &[u8],
    resolver: &NameResolver,
    uppers: &mut Vec<String>,
) {
    for i in 0..clause.named_child_count() {
        if let Some(child) = clause.named_child(i) {
            match child.kind() {
                "name" | "qualified_name" | "namespace_name" => {
                    let raw_name = node_text(child, source);
                    let resolved = resolver.resolve_class(raw_name);
                    uppers.push(resolved);
                }
                "named_type" => {
                    // named_type wraps a name or qualified_name
                    if let Some(inner) = child.named_child(0) {
                        let raw_name = node_text(inner, source);
                        let resolved = resolver.resolve_class(raw_name);
                        uppers.push(resolved);
                    }
                }
                _ => {}
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Class body collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect members from a class/interface/trait/enum body (declaration_list or enum_declaration_list).
fn collect_class_body(
    body: Node,
    source: &[u8],
    resolver: &mut NameResolver,
    db: &TypeDatabase,
    class_fqn: &str,
) {
    for i in 0..body.named_child_count() {
        if let Some(child) = body.named_child(i) {
            match child.kind() {
                "method_declaration" => {
                    collect_method(child, source, resolver, db, class_fqn);
                }
                "property_declaration" => {
                    collect_property(child, source, db, class_fqn);
                }
                "use_declaration" => {
                    // Trait use statement: `use SomeTrait, AnotherTrait;`
                    collect_trait_use(child, source, resolver, db, class_fqn);
                }
                "enum_case" => {
                    // Enum cases are not methods/properties, but could be tracked.
                    // For now, we skip them since they don't affect method resolution.
                }
                "const_declaration" => {
                    // Class constants — we don't track these in TypeDatabase currently
                }
                _ => {}
            }
        }
    }
}

/// Collect a trait `use` statement and add the trait(s) to uppers.
fn collect_trait_use(
    node: Node,
    source: &[u8],
    resolver: &NameResolver,
    db: &TypeDatabase,
    class_fqn: &str,
) {
    let mut trait_names = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            match child.kind() {
                "name" | "qualified_name" | "namespace_name" => {
                    let raw_name = node_text(child, source);
                    let resolved = resolver.resolve_class(raw_name);
                    trait_names.push(resolved);
                }
                _ => {}
            }
        }
    }

    if !trait_names.is_empty() {
        // Merge with existing uppers (DashMap entry API)
        db.uppers
            .entry(class_fqn.to_string())
            .or_default()
            .extend(trait_names);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Method collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect a method declaration.
fn collect_method(
    node: Node,
    source: &[u8],
    _resolver: &mut NameResolver,
    db: &TypeDatabase,
    class_fqn: &str,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, source).to_string(),
        None => return,
    };

    // Return type
    let return_type = node
        .child_by_field_name("return_type")
        .map(|rt| extract_type_string(rt, source));

    // Parameters
    let params = match node.child_by_field_name("parameters") {
        Some(params_node) => collect_params(params_node, source),
        None => Vec::new(),
    };

    // Handle promoted constructor parameters as properties
    if name == "__construct" {
        for param in &params {
            if param.is_promoted {
                db.add_property(class_fqn, &param.name, param.type_hint.clone());
            }
        }
    }

    db.add_method(class_fqn, &name, return_type, params);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Parameter collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect parameters from a `formal_parameters` node.
fn collect_params(params_node: Node, source: &[u8]) -> Vec<ParamInfo> {
    let mut params = Vec::new();

    for i in 0..params_node.named_child_count() {
        if let Some(child) = params_node.named_child(i) {
            match child.kind() {
                "simple_parameter" => {
                    if let Some(param) = parse_simple_parameter(child, source) {
                        params.push(param);
                    }
                }
                "variadic_parameter" => {
                    if let Some(param) = parse_variadic_parameter(child, source) {
                        params.push(param);
                    }
                }
                "property_promotion_parameter" => {
                    if let Some(param) = parse_promoted_parameter(child, source) {
                        params.push(param);
                    }
                }
                _ => {}
            }
        }
    }

    params
}

/// Parse a simple_parameter node.
fn parse_simple_parameter(node: Node, source: &[u8]) -> Option<ParamInfo> {
    let name = extract_param_name(node, source)?;

    let type_hint = extract_param_type(node, source);
    let has_default = has_default_value(node, source);

    Some(ParamInfo {
        name,
        type_hint,
        has_default,
        is_variadic: false,
        is_promoted: false,
        is_readonly: false,
    })
}

/// Parse a variadic_parameter node (`...$param`).
fn parse_variadic_parameter(node: Node, source: &[u8]) -> Option<ParamInfo> {
    let name = extract_param_name(node, source)?;
    let type_hint = extract_param_type(node, source);

    Some(ParamInfo {
        name,
        type_hint,
        has_default: false,
        is_variadic: true,
        is_promoted: false,
        is_readonly: false,
    })
}

/// Parse a property_promotion_parameter node (constructor promotion).
fn parse_promoted_parameter(node: Node, source: &[u8]) -> Option<ParamInfo> {
    let name = extract_param_name(node, source)?;
    let type_hint = extract_param_type(node, source);
    let has_default = has_default_value(node, source);
    let modifiers = collect_modifiers(node, source);
    let is_readonly = modifiers.contains(&"readonly");

    Some(ParamInfo {
        name,
        type_hint,
        has_default,
        is_variadic: false,
        is_promoted: true,
        is_readonly,
    })
}

/// Extract the parameter name from a parameter node.
/// Looks for a `variable_name` child and strips the `$` prefix.
fn extract_param_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(var_node) = child_by_kind(node, "variable_name") {
        let text = node_text(var_node, source);
        // Strip leading `$`
        let name = text.strip_prefix('$').unwrap_or(text);
        return Some(name.to_string());
    }
    None
}

/// Extract the type hint from a parameter node.
/// Checks for type-related children before the variable_name.
fn extract_param_type(node: Node, source: &[u8]) -> Option<String> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            match child.kind() {
                "primitive_type" | "named_type" | "optional_type" | "union_type"
                | "intersection_type" | "nullable_type" => {
                    return Some(extract_type_string(child, source));
                }
                "name" | "qualified_name" | "namespace_name" => {
                    // Sometimes the type is just a bare name
                    // But make sure it's not the parameter name
                    if child_by_kind(node, "variable_name")
                        .map(|v| child.end_byte() < v.start_byte())
                        .unwrap_or(false)
                    {
                        return Some(node_text(child, source).to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Check if a parameter has a default value.
/// Looks for an `=` anonymous child followed by an expression.
fn has_default_value(node: Node, source: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() && node_text(child, source) == "=" {
                return true;
            }
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════════════
// Property collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect properties from a `property_declaration` node.
fn collect_property(node: Node, source: &[u8], db: &TypeDatabase, class_fqn: &str) {
    // Extract type hint (appears before property_element nodes)
    let type_hint = extract_property_type(node, source);

    // Property declarations can have multiple property elements: `public int $a, $b;`
    let elements = children_by_kind(node, "property_element");
    for element in elements {
        if let Some(var_node) = child_by_kind(element, "variable_name") {
            let var_text = node_text(var_node, source);
            let name = var_text.strip_prefix('$').unwrap_or(var_text);
            db.add_property(class_fqn, name, type_hint.clone());
        }
    }
}

/// Extract the type hint from a property_declaration node.
fn extract_property_type(node: Node, source: &[u8]) -> Option<String> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            match child.kind() {
                "primitive_type" | "named_type" | "optional_type" | "union_type"
                | "intersection_type" | "nullable_type" => {
                    return Some(extract_type_string(child, source));
                }
                "property_element" => {
                    // We've gone past the type into the properties
                    return None;
                }
                _ => {}
            }
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// Function collection
// ═══════════════════════════════════════════════════════════════════════════════

/// Collect a standalone function definition.
fn collect_function_def(
    node: Node,
    source: &[u8],
    resolver: &mut NameResolver,
    db: &TypeDatabase,
    file_path: &Path,
) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, source),
        None => return,
    };

    let fqn = qualify_definition(resolver, name);

    // Return type
    let return_type = node
        .child_by_field_name("return_type")
        .map(|rt| extract_type_string(rt, source));

    // Parameters
    let params = match node.child_by_field_name("parameters") {
        Some(params_node) => collect_params(params_node, source),
        None => Vec::new(),
    };

    let docblock = preceding_doc_comment(node, source).map(|s| s.to_string());

    // Insert the function as a type definition
    let def = TypeDef {
        kind: SymbolKind::Function,
        file_path: file_path.to_owned(),
        is_abstract: false,
        is_final: false,
        is_readonly: false,
        enum_backing_type: None,
        docblock,
    };
    db.insert_def(&fqn, def);

    // Store function return type (DashMap insert takes &self)
    db.function_return_types.insert(fqn.clone(), return_type);

    // Store function params using FQN as the "class" part
    // This allows looking them up in a consistent way
    let key = format!("{}::", fqn);
    db.method_params.insert(key, params);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Qualify a definition name with the current namespace.
fn qualify_definition(resolver: &NameResolver, name: &str) -> String {
    let ns = resolver.namespace();
    if ns.is_empty() {
        name.to_string()
    } else {
        format!("{}\\{}", ns, name)
    }
}

/// Extract a type string from a type node.
///
/// Handles: primitive_type, named_type, optional_type (?Type),
/// union_type (A|B), intersection_type (A&B), nullable_type.
pub fn extract_type_string(type_node: Node, source: &[u8]) -> String {
    // For simple types, just return the text
    match type_node.kind() {
        "primitive_type" | "name" | "qualified_name" | "namespace_name" => {
            node_text(type_node, source).to_string()
        }
        "named_type" => {
            // named_type wraps a name/qualified_name
            node_text(type_node, source).to_string()
        }
        "optional_type" | "nullable_type" => {
            // ?Type — reconstruct the full string
            node_text(type_node, source).to_string()
        }
        "union_type" => {
            // A|B|C — the text includes the pipes
            node_text(type_node, source).to_string()
        }
        "intersection_type" => {
            // A&B — the text includes the ampersands
            node_text(type_node, source).to_string()
        }
        _ => {
            // Fallback: use the raw text
            node_text(type_node, source).to_string()
        }
    }
}

/// Extract the preceding docblock comment for a node.
fn _extract_preceding_docblock<'a>(node: Node<'a>, source: &'a [u8]) -> Option<String> {
    preceding_doc_comment(node, source).map(|s| s.to_string())
}

/// Collect modifiers from a node's children (both named and anonymous).
///
/// Returns a list of modifier strings found: "abstract", "final", "static",
/// "readonly", "public", "protected", "private".
fn collect_modifiers<'a>(node: Node<'a>, source: &'a [u8]) -> Vec<&'a str> {
    let mut modifiers = Vec::new();

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "abstract_modifier" => modifiers.push("abstract"),
                "final_modifier" => modifiers.push("final"),
                "readonly_modifier" => modifiers.push("readonly"),
                "visibility_modifier" => {
                    let text = node_text(child, source);
                    match text {
                        "public" => modifiers.push("public"),
                        "protected" => modifiers.push("protected"),
                        "private" => modifiers.push("private"),
                        _ => {}
                    }
                }
                "static_modifier" => modifiers.push("static"),
                _ => {
                    // Also check for anonymous keyword nodes
                    if !child.is_named() {
                        let text = node_text(child, source);
                        match text {
                            "abstract" | "final" | "static" | "readonly" => {
                                modifiers.push(text);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    modifiers
}

/// Extract visibility from modifiers.
#[allow(dead_code)]
fn extract_visibility(modifiers: &[&str]) -> Visibility {
    if modifiers.contains(&"private") {
        Visibility::Private
    } else if modifiers.contains(&"protected") {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::parser::PhpParser;

    fn collect_from_source(source: &str) -> TypeDatabase {
        let mut parser = PhpParser::new();
        let parsed = parser.parse(source, "test.php").unwrap();
        let db = TypeDatabase::new();
        collect_defs_from_file(&parsed.path, &parsed.source, &parsed.tree, &db);
        db
    }

    #[test]
    fn test_collect_simple_class() {
        let db = collect_from_source(
            r#"<?php
class User {
    public string $name;

    public function getName(): string {
        return $this->name;
    }
}
"#,
        );

        assert!(db.has_class("User"));
        let def = db.get_def("User").unwrap();
        assert_eq!(def.kind, SymbolKind::Class);
        assert!(!def.is_abstract);
        assert!(!def.is_final);

        // Method
        assert!(db.get_method_params("User", "getName").is_some());
        assert_eq!(
            db.get_method_return_type("User", "getName").as_deref(),
            Some("string")
        );

        // Property
        assert_eq!(
            db.get_property_type("User", "name").as_deref(),
            Some("string")
        );
    }

    #[test]
    fn test_collect_namespaced_class() {
        let db = collect_from_source(
            r#"<?php
namespace App\Models;

use App\Base\BaseModel;

class User extends BaseModel {
    public function getRole(): ?string {
        return null;
    }
}
"#,
        );

        assert!(db.has_class("App\\Models\\User"));
        let def = db.get_def("App\\Models\\User").unwrap();
        assert_eq!(def.kind, SymbolKind::Class);

        // Check uppers
        let uppers = db.get_direct_uppers("App\\Models\\User");
        assert_eq!(uppers, &["App\\Base\\BaseModel"]);

        // Method
        assert_eq!(
            db.get_method_return_type("App\\Models\\User", "getRole").as_deref(),
            Some("?string")
        );
    }

    #[test]
    fn test_collect_abstract_class() {
        let db = collect_from_source(
            r#"<?php
abstract class BaseService {
    abstract public function execute(): void;

    public function log(string $msg): void {}
}
"#,
        );

        assert!(db.has_class("BaseService"));
        let def = db.get_def("BaseService").unwrap();
        assert_eq!(def.kind, SymbolKind::AbstractClass);
        assert!(def.is_abstract);

        assert!(db.get_method_params("BaseService", "execute").is_some());
        assert!(db.get_method_params("BaseService", "log").is_some());
    }

    #[test]
    fn test_collect_final_class() {
        let db = collect_from_source("<?php\nfinal class Singleton {}");

        let def = db.get_def("Singleton").unwrap();
        assert_eq!(def.kind, SymbolKind::FinalClass);
        assert!(def.is_final);
    }

    #[test]
    fn test_collect_interface() {
        let db = collect_from_source(
            r#"<?php
namespace App\Contracts;

interface HasRole {
    public function getRole(): string;
}
"#,
        );

        assert!(db.has_class("App\\Contracts\\HasRole"));
        let def = db.get_def("App\\Contracts\\HasRole").unwrap();
        assert_eq!(def.kind, SymbolKind::Interface);

        assert!(db.get_method_params("App\\Contracts\\HasRole", "getRole").is_some());
    }

    #[test]
    fn test_collect_trait() {
        let db = collect_from_source(
            r#"<?php
trait Loggable {
    public string $logLevel;

    public function log(string $msg): void {}
}
"#,
        );

        assert!(db.has_class("Loggable"));
        let def = db.get_def("Loggable").unwrap();
        assert_eq!(def.kind, SymbolKind::Trait);

        assert!(db.get_method_params("Loggable", "log").is_some());
        assert_eq!(
            db.get_property_type("Loggable", "logLevel").as_deref(),
            Some("string")
        );
    }

    #[test]
    fn test_collect_enum() {
        let db = collect_from_source(
            r#"<?php
enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';

    public function label(): string {
        return $this->value;
    }
}
"#,
        );

        assert!(db.has_class("Status"));
        let def = db.get_def("Status").unwrap();
        assert_eq!(def.kind, SymbolKind::Enum);
        assert_eq!(def.enum_backing_type.as_deref(), Some("string"));

        assert!(db.get_method_params("Status", "label").is_some());
    }

    #[test]
    fn test_collect_enum_without_backing() {
        let db = collect_from_source(
            r#"<?php
enum Suit {
    case Hearts;
    case Diamonds;
}
"#,
        );

        let def = db.get_def("Suit").unwrap();
        assert_eq!(def.kind, SymbolKind::Enum);
        assert!(def.enum_backing_type.is_none());
    }

    #[test]
    fn test_collect_function() {
        let db = collect_from_source(
            r#"<?php
namespace App\Helpers;

function formatName(string $first, string $last): string {
    return "$first $last";
}
"#,
        );

        assert!(db.has_class("App\\Helpers\\formatName"));
        let def = db.get_def("App\\Helpers\\formatName").unwrap();
        assert_eq!(def.kind, SymbolKind::Function);

        assert_eq!(
            db.function_return_types
                .get("App\\Helpers\\formatName")
                .map(|r| r.value().clone()),
            Some(Some("string".to_string()))
        );
    }

    #[test]
    fn test_collect_method_params() {
        let db = collect_from_source(
            r#"<?php
class Foo {
    public function bar(int $a, string $b = 'default', float ...$rest): void {}
}
"#,
        );

        let params = db.get_method_params("Foo", "bar").unwrap();
        assert_eq!(params.len(), 3);

        assert_eq!(params[0].name, "a");
        assert_eq!(params[0].type_hint.as_deref(), Some("int"));
        assert!(!params[0].has_default);
        assert!(!params[0].is_variadic);

        assert_eq!(params[1].name, "b");
        assert_eq!(params[1].type_hint.as_deref(), Some("string"));
        assert!(params[1].has_default);
        assert!(!params[1].is_variadic);

        assert_eq!(params[2].name, "rest");
        assert_eq!(params[2].type_hint.as_deref(), Some("float"));
        assert!(!params[2].has_default);
        assert!(params[2].is_variadic);
    }

    #[test]
    fn test_collect_constructor_promotion() {
        let db = collect_from_source(
            r#"<?php
class User {
    public function __construct(
        private readonly string $email,
        protected int $age = 0,
    ) {}
}
"#,
        );

        let params = db.get_method_params("User", "__construct").unwrap();
        assert_eq!(params.len(), 2);

        assert_eq!(params[0].name, "email");
        assert!(params[0].is_promoted);
        assert!(params[0].is_readonly);
        assert_eq!(params[0].type_hint.as_deref(), Some("string"));

        assert_eq!(params[1].name, "age");
        assert!(params[1].is_promoted);
        assert!(!params[1].is_readonly);
        assert!(params[1].has_default);

        // Promoted params should also create properties
        assert_eq!(
            db.get_property_type("User", "email").as_deref(),
            Some("string")
        );
        assert_eq!(
            db.get_property_type("User", "age").as_deref(),
            Some("int")
        );
    }

    #[test]
    fn test_collect_class_with_implements() {
        let db = collect_from_source(
            r#"<?php
namespace App;

use App\Contracts\HasRole;
use App\Contracts\Cacheable;

class User implements HasRole, Cacheable {}
"#,
        );

        let uppers = db.get_direct_uppers("App\\User");
        assert_eq!(uppers.len(), 2);
        assert!(uppers.contains(&"App\\Contracts\\HasRole".to_string()));
        assert!(uppers.contains(&"App\\Contracts\\Cacheable".to_string()));
    }

    #[test]
    fn test_collect_class_with_trait_use() {
        let db = collect_from_source(
            r#"<?php
namespace App;

use App\Traits\Loggable;

class User {
    use Loggable;
}
"#,
        );

        let uppers = db.get_direct_uppers("App\\User");
        assert!(uppers.contains(&"App\\Traits\\Loggable".to_string()));
    }

    #[test]
    fn test_collect_class_extends_and_implements() {
        let db = collect_from_source(
            r#"<?php
use App\Base\Model;
use App\Contracts\HasRole;

class User extends Model implements HasRole {}
"#,
        );

        let uppers = db.get_direct_uppers("User");
        assert_eq!(uppers.len(), 2);
        assert_eq!(uppers[0], "App\\Base\\Model");
        assert_eq!(uppers[1], "App\\Contracts\\HasRole");
    }

    #[test]
    fn test_collect_interface_extends() {
        let db = collect_from_source(
            r#"<?php
namespace App\Contracts;

interface CacheableRole extends HasRole {}
"#,
        );

        // Interfaces use base_clause for extends too
        // In this case HasRole is unimported so it resolves to App\Contracts\HasRole
        let uppers = db.get_direct_uppers("App\\Contracts\\CacheableRole");
        assert_eq!(uppers.len(), 1);
        assert_eq!(uppers[0], "App\\Contracts\\HasRole");
    }

    #[test]
    fn test_collect_multiple_properties() {
        let db = collect_from_source(
            r#"<?php
class Foo {
    public string $a;
    protected ?int $b;
    private $c;
}
"#,
        );

        assert_eq!(db.get_property_type("Foo", "a").as_deref(), Some("string"));
        assert_eq!(db.get_property_type("Foo", "b").as_deref(), Some("?int"));
        // $c has no type hint
        assert!(db.property_types.contains_key("Foo::$c"));
        assert_eq!(
            db.property_types.get("Foo::$c").map(|r| r.value().clone()),
            Some(None)
        );
    }

    #[test]
    fn test_collect_union_type_property() {
        let db = collect_from_source(
            r#"<?php
class Foo {
    public int|string $value;
}
"#,
        );

        assert_eq!(
            db.get_property_type("Foo", "value").as_deref(),
            Some("int|string")
        );
    }

    #[test]
    fn test_collect_optional_return_type() {
        let db = collect_from_source(
            r#"<?php
class Foo {
    public function bar(): ?string {}
}
"#,
        );

        assert_eq!(
            db.get_method_return_type("Foo", "bar").as_deref(),
            Some("?string")
        );
    }

    #[test]
    fn test_collect_docblock() {
        let db = collect_from_source(
            r#"<?php
/** This is a doc comment for User */
class User {}
"#,
        );

        let def = db.get_def("User").unwrap();
        assert!(def.docblock.is_some());
        assert!(def.docblock.as_ref().unwrap().contains("doc comment for User"));
    }

    #[test]
    fn test_collect_braced_namespace() {
        let db = collect_from_source(
            r#"<?php
namespace App\Models {
    class User {}
}
"#,
        );

        assert!(db.has_class("App\\Models\\User"));
    }

    #[test]
    fn test_collect_global_namespace_class() {
        let db = collect_from_source("<?php\nclass Foo {}");

        assert!(db.has_class("Foo"));
        let def = db.get_def("Foo").unwrap();
        assert_eq!(def.kind, SymbolKind::Class);
        assert_eq!(def.file_path, PathBuf::from("test.php"));
    }

    #[test]
    fn test_collect_fqn_in_implements() {
        let db = collect_from_source(
            r#"<?php
class User implements \Serializable {}
"#,
        );

        let uppers = db.get_direct_uppers("User");
        assert_eq!(uppers, &["Serializable"]);
    }

    #[test]
    fn test_collect_no_return_type() {
        let db = collect_from_source(
            r#"<?php
class Foo {
    public function bar() {}
}
"#,
        );

        // No return type declared
        assert_eq!(db.get_method_return_type("Foo", "bar"), None);
    }

    #[test]
    fn test_collect_multiple_classes_in_file() {
        let db = collect_from_source(
            r#"<?php
namespace App;

class First {}
class Second {}
interface Third {}
"#,
        );

        assert!(db.has_class("App\\First"));
        assert!(db.has_class("App\\Second"));
        assert!(db.has_class("App\\Third"));
    }
}
