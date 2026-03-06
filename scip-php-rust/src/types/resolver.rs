//! Variable type tracking and expression type resolution.
//!
//! `VariableTypeMap` tracks variable-to-type mappings per callable scope.
//! `resolve_expr_type` resolves the PHP type of a CST expression node.

use rustc_hash::FxHashMap;

use crate::names::resolver::NameResolver;
use crate::parser::cst::node_text;
use crate::symbol::scope::ScopeStack;
use crate::types::TypeDatabase;

// ═══════════════════════════════════════════════════════════════════════════════
// VariableTypeMap
// ═══════════════════════════════════════════════════════════════════════════════

/// Maps variable names (without $) to their PHP type FQN.
/// Scoped per callable (method, function, closure, arrow function).
pub struct VariableTypeMap {
    vars: FxHashMap<String, String>,
}

impl VariableTypeMap {
    pub fn new() -> Self {
        Self {
            vars: FxHashMap::default(),
        }
    }

    /// Record that variable $name has the given PHP type FQN.
    pub fn set(&mut self, var_name: &str, type_fqn: String) {
        let name = var_name.trim_start_matches('$');
        if !name.is_empty() && !type_fqn.is_empty() {
            self.vars.insert(name.to_string(), type_fqn);
        }
    }

    /// Get the type of variable $name, if known.
    pub fn get(&self, var_name: &str) -> Option<&str> {
        let name = var_name.trim_start_matches('$');
        self.vars.get(name).map(|s| s.as_str())
    }

    /// Clear the map (e.g., when entering a new callable scope).
    pub fn clear(&mut self) {
        self.vars.clear();
    }
}

impl Default for VariableTypeMap {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Strip nullable prefix from a type FQN.
/// "?App\\Foo" -> "App\\Foo", "App\\Foo" -> "App\\Foo"
pub fn strip_nullable(fqn: &str) -> &str {
    fqn.trim_start_matches('?')
}

/// Resolve a scope keyword or class name to a FQN.
fn resolve_scope_to_fqn(
    scope_text: &str,
    scope: &ScopeStack,
    type_db: &TypeDatabase,
    resolver: &NameResolver,
) -> Option<String> {
    match scope_text {
        "self" | "static" => scope.current_class().map(|s| s.to_string()),
        "parent" => {
            // Look up the first direct upper (parent class) of the current class
            let class_fqn = scope.current_class()?;
            let uppers = type_db.get_direct_uppers(class_fqn);
            uppers.first().map(|s| s.to_string())
        }
        name => {
            let fqn = resolver.resolve_class(name);
            if fqn.is_empty() {
                None
            } else {
                Some(fqn)
            }
        }
    }
}

/// Resolve a type string (which may be "self", "static", "$this", or a class name)
/// in the context of a specific class FQN.
fn resolve_type_string_to_fqn(
    type_str: &str,
    current_class_fqn: &str,
    scope: &ScopeStack,
    type_db: &TypeDatabase,
    resolver: &NameResolver,
) -> Option<String> {
    match type_str {
        "self" | "static" | "$this" => Some(current_class_fqn.to_string()),
        "parent" => {
            let class_fqn = scope.current_class()?;
            let uppers = type_db.get_direct_uppers(class_fqn);
            uppers.first().map(|s| s.to_string())
        }
        "void" | "never" | "null" | "bool" | "int" | "float" | "string" | "array" | "object"
        | "mixed" | "iterable" | "callable" | "true" | "false" => None,
        name if name.starts_with('?') => {
            resolve_type_string_to_fqn(&name[1..], current_class_fqn, scope, type_db, resolver)
        }
        name => {
            if name.contains('\\') {
                // Already fully qualified
                Some(name.to_string())
            } else {
                let fqn = resolver.resolve_class(name);
                if fqn.is_empty() {
                    None
                } else {
                    Some(fqn)
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// resolve_expr_type
// ═══════════════════════════════════════════════════════════════════════════════

/// Resolve the PHP type of a CST expression node.
///
/// Parameters are passed individually (not as `&IndexingContext`) to avoid
/// borrow checker conflicts when the caller holds `&mut IndexingContext`.
pub fn resolve_expr_type(
    node: tree_sitter::Node,
    source: &[u8],
    var_types: &VariableTypeMap,
    scope: &ScopeStack,
    type_db: &TypeDatabase,
    resolver: &NameResolver,
) -> Option<String> {
    match node.kind() {
        // Case 1: Variable — look up in var_types, handle $this specially
        "variable_name" => {
            let var_text = node.utf8_text(source).unwrap_or("");
            if var_text == "$this" {
                scope.current_class().map(|s| s.to_string())
            } else {
                var_types.get(var_text).map(|s| s.to_string())
            }
        }

        // Case 2: new Foo() — resolve class name
        "object_creation_expression" => {
            // The class node is the first named child (a name/qualified_name)
            // tree-sitter-php uses no field name for the class in object_creation_expression
            // We look for the first named child that is a name-like node
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    match child.kind() {
                        "name" | "qualified_name" | "namespace_name" => {
                            let class_text = node_text(child, source);
                            return match class_text {
                                "self" | "static" => {
                                    scope.current_class().map(|s| s.to_string())
                                }
                                "parent" => None,
                                name => {
                                    let fqn = resolver.resolve_class(name);
                                    if fqn.is_empty() {
                                        None
                                    } else {
                                        Some(fqn)
                                    }
                                }
                            };
                        }
                        _ => {}
                    }
                }
            }
            None
        }

        // Case 3 & 4: $obj->method() and $obj?->method()
        "member_call_expression" | "nullsafe_member_call_expression" => {
            let obj_node = node.child_by_field_name("object")?;
            let method_name_node = node.child_by_field_name("name")?;
            let method_name = method_name_node.utf8_text(source).ok()?;

            let obj_type = resolve_expr_type(obj_node, source, var_types, scope, type_db, resolver)?;
            let obj_type_clean = strip_nullable(&obj_type);

            let return_type = type_db.resolve_method_return_type(obj_type_clean, method_name)?;
            resolve_type_string_to_fqn(return_type, obj_type_clean, scope, type_db, resolver)
        }

        // Case 5: Foo::method()
        "scoped_call_expression" => {
            let scope_node = node.child_by_field_name("scope")?;
            let method_node = node.child_by_field_name("name")?;
            let scope_text = scope_node.utf8_text(source).unwrap_or("");
            let method_name = method_node.utf8_text(source).unwrap_or("");

            let class_fqn = resolve_scope_to_fqn(scope_text, scope, type_db, resolver)?;
            let return_type = type_db.resolve_method_return_type(&class_fqn, method_name)?;
            resolve_type_string_to_fqn(return_type, &class_fqn, scope, type_db, resolver)
        }

        // Case 6 & 7: $obj->prop and $obj?->prop
        "member_access_expression" | "nullsafe_member_access_expression" => {
            // Only resolve type if this is NOT a method call (no arguments child)
            // Actually, member_access_expression is the property access form; method calls
            // use member_call_expression. So this is always property access.
            let obj_node = node.child_by_field_name("object")?;
            let prop_name_node = node.child_by_field_name("name")?;
            let prop_name = prop_name_node.utf8_text(source).ok()?;

            let obj_type = resolve_expr_type(obj_node, source, var_types, scope, type_db, resolver)?;
            let obj_type_clean = strip_nullable(&obj_type);

            let prop_type = type_db.resolve_property_type(obj_type_clean, prop_name)?;
            resolve_type_string_to_fqn(prop_type, obj_type_clean, scope, type_db, resolver)
        }

        // Case 8: Foo::$prop
        "scoped_property_access_expression" => {
            let scope_node = node.child_by_field_name("scope")?;
            let prop_node = node.child_by_field_name("name")?;
            let scope_text = scope_node.utf8_text(source).unwrap_or("");
            let prop_name = prop_node
                .utf8_text(source)
                .unwrap_or("")
                .trim_start_matches('$');

            let class_fqn = resolve_scope_to_fqn(scope_text, scope, type_db, resolver)?;
            let prop_type = type_db.resolve_property_type(&class_fqn, prop_name)?;
            resolve_type_string_to_fqn(prop_type, &class_fqn, scope, type_db, resolver)
        }

        // Case 9: $arr[$key] — mostly unknown
        "subscript_expression" => None,

        // Case 10: $a ?? $b
        "binary_expression" => {
            // Check for ?? operator
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = op.utf8_text(source).unwrap_or("");
                if op_text == "??" {
                    if let Some(left) = node.child_by_field_name("left") {
                        let left_type =
                            resolve_expr_type(left, source, var_types, scope, type_db, resolver);
                        if left_type.is_some() {
                            return left_type;
                        }
                    }
                    if let Some(right) = node.child_by_field_name("right") {
                        return resolve_expr_type(
                            right, source, var_types, scope, type_db, resolver,
                        );
                    }
                }
            }
            None
        }

        // Case 11: $cond ? $a : $b
        "conditional_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                let body_type =
                    resolve_expr_type(body, source, var_types, scope, type_db, resolver);
                if body_type.is_some() {
                    return body_type;
                }
            }
            if let Some(alt) = node.child_by_field_name("alternative") {
                return resolve_expr_type(alt, source, var_types, scope, type_db, resolver);
            }
            None
        }

        // Case 12: match ($x) { ... }
        "match_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                // First match arm
                if let Some(first_arm) = body.named_child(0) {
                    // match_conditional_expression has a "body" field for the return value
                    if let Some(arm_value) = first_arm.child_by_field_name("return_expression") {
                        return resolve_expr_type(
                            arm_value, source, var_types, scope, type_db, resolver,
                        );
                    }
                    // Fallback: try "body" field
                    if let Some(arm_value) = first_arm.child_by_field_name("body") {
                        return resolve_expr_type(
                            arm_value, source, var_types, scope, type_db, resolver,
                        );
                    }
                }
            }
            None
        }

        // Case 13: fn() => expr
        "arrow_function" => {
            let body = node.child_by_field_name("body")?;
            resolve_expr_type(body, source, var_types, scope, type_db, resolver)
        }

        // Parenthesized expression — unwrap
        "parenthesized_expression" => {
            // Look for the inner expression
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    let result =
                        resolve_expr_type(child, source, var_types, scope, type_db, resolver);
                    if result.is_some() {
                        return result;
                    }
                }
            }
            None
        }

        // Unknown expression — no type info
        _ => None,
    }
}

/// Resolve a type hint CST node to an FQN string.
/// Used to extract FQNs from parameter type hints.
/// Returns empty string for primitives or unresolvable types.
pub fn resolve_type_node_to_fqn(
    type_node: tree_sitter::Node,
    source: &[u8],
    resolver: &NameResolver,
) -> String {
    match type_node.kind() {
        "named_type" => {
            // named_type wraps a name/qualified_name child
            if let Some(child) = type_node.named_child(0) {
                resolve_type_node_to_fqn(child, source, resolver)
            } else {
                String::new()
            }
        }
        "name" | "qualified_name" | "namespace_name" => {
            let text = node_text(type_node, source);
            if crate::indexing::references::is_primitive_type(text) {
                return String::new();
            }
            resolver.resolve_class(text)
        }
        "optional_type" => {
            // ?Foo -> resolve Foo
            if let Some(inner) = type_node.named_child(0) {
                resolve_type_node_to_fqn(inner, source, resolver)
            } else {
                String::new()
            }
        }
        "union_type" => {
            // Use the first non-primitive type
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    let resolved = resolve_type_node_to_fqn(child, source, resolver);
                    if !resolved.is_empty() {
                        return resolved;
                    }
                }
            }
            String::new()
        }
        "intersection_type" => {
            // Use the first type
            if let Some(child) = type_node.named_child(0) {
                resolve_type_node_to_fqn(child, source, resolver)
            } else {
                String::new()
            }
        }
        // Primitive types
        "primitive_type" | "null" | "bottom_type" => String::new(),
        _ => String::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_type_map_basic() {
        let mut map = VariableTypeMap::new();
        assert!(map.get("foo").is_none());

        map.set("$foo", "App\\Foo".to_string());
        assert_eq!(map.get("$foo"), Some("App\\Foo"));
        assert_eq!(map.get("foo"), Some("App\\Foo"));

        map.set("bar", "App\\Bar".to_string());
        assert_eq!(map.get("bar"), Some("App\\Bar"));
        assert_eq!(map.get("$bar"), Some("App\\Bar"));
    }

    #[test]
    fn test_variable_type_map_skip_empty() {
        let mut map = VariableTypeMap::new();
        map.set("$foo", String::new());
        assert!(map.get("foo").is_none());
    }

    #[test]
    fn test_variable_type_map_clear() {
        let mut map = VariableTypeMap::new();
        map.set("$foo", "App\\Foo".to_string());
        assert!(map.get("foo").is_some());

        map.clear();
        assert!(map.get("foo").is_none());
    }

    #[test]
    fn test_variable_type_map_overwrite() {
        let mut map = VariableTypeMap::new();
        map.set("$x", "App\\Foo".to_string());
        assert_eq!(map.get("x"), Some("App\\Foo"));

        map.set("$x", "App\\Bar".to_string());
        assert_eq!(map.get("x"), Some("App\\Bar"));
    }

    #[test]
    fn test_strip_nullable() {
        assert_eq!(strip_nullable("?App\\Foo"), "App\\Foo");
        assert_eq!(strip_nullable("App\\Foo"), "App\\Foo");
        assert_eq!(strip_nullable("?string"), "string");
        assert_eq!(strip_nullable(""), "");
    }

    #[test]
    fn test_resolve_scope_to_fqn_self() {
        let mut scope = ScopeStack::new();
        scope.push_class(
            "App\\Foo".to_string(),
            crate::symbol::scope::ClassKind::Class,
        );
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();
        assert_eq!(
            resolve_scope_to_fqn("self", &scope, &type_db, &resolver),
            Some("App\\Foo".to_string())
        );
        assert_eq!(
            resolve_scope_to_fqn("static", &scope, &type_db, &resolver),
            Some("App\\Foo".to_string())
        );
    }

    #[test]
    fn test_resolve_scope_to_fqn_class_name() {
        let scope = ScopeStack::new();
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();
        let result = resolve_scope_to_fqn("SomeClass", &scope, &type_db, &resolver);
        // Without namespace, resolves to just "SomeClass"
        assert!(result.is_some());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_resolve_scope_to_fqn_parent() {
        let mut scope = ScopeStack::new();
        scope.push_class(
            "App\\Child".to_string(),
            crate::symbol::scope::ClassKind::Class,
        );
        let mut type_db = TypeDatabase::new();
        type_db.add_uppers("App\\Child", vec!["App\\Base".to_string()]);
        let resolver = NameResolver::new();
        assert_eq!(
            resolve_scope_to_fqn("parent", &scope, &type_db, &resolver),
            Some("App\\Base".to_string())
        );
    }

    #[test]
    fn test_resolve_scope_to_fqn_parent_no_uppers() {
        let mut scope = ScopeStack::new();
        scope.push_class(
            "App\\Root".to_string(),
            crate::symbol::scope::ClassKind::Class,
        );
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();
        assert_eq!(
            resolve_scope_to_fqn("parent", &scope, &type_db, &resolver),
            None
        );
    }

    #[test]
    fn test_resolve_type_string_self_static_this() {
        let scope = ScopeStack::new();
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();

        // self/static/$this should resolve to the given class FQN
        assert_eq!(
            resolve_type_string_to_fqn("self", "App\\Builder", &scope, &type_db, &resolver),
            Some("App\\Builder".to_string())
        );
        assert_eq!(
            resolve_type_string_to_fqn("static", "App\\Builder", &scope, &type_db, &resolver),
            Some("App\\Builder".to_string())
        );
        assert_eq!(
            resolve_type_string_to_fqn("$this", "App\\Builder", &scope, &type_db, &resolver),
            Some("App\\Builder".to_string())
        );
    }

    #[test]
    fn test_resolve_type_string_primitives_return_none() {
        let scope = ScopeStack::new();
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();

        for prim in &["void", "int", "string", "bool", "float", "array", "null", "mixed"] {
            assert_eq!(
                resolve_type_string_to_fqn(prim, "App\\Foo", &scope, &type_db, &resolver),
                None,
                "Expected None for primitive type '{}'", prim
            );
        }
    }

    #[test]
    fn test_resolve_type_string_nullable() {
        let scope = ScopeStack::new();
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();

        // ?self should strip nullable and resolve
        assert_eq!(
            resolve_type_string_to_fqn("?self", "App\\Builder", &scope, &type_db, &resolver),
            Some("App\\Builder".to_string())
        );
    }

    #[test]
    fn test_resolve_type_string_fqn() {
        let scope = ScopeStack::new();
        let type_db = TypeDatabase::new();
        let resolver = NameResolver::new();

        // Already-qualified names should pass through
        assert_eq!(
            resolve_type_string_to_fqn("App\\Models\\User", "App\\Foo", &scope, &type_db, &resolver),
            Some("App\\Models\\User".to_string())
        );
    }

    #[test]
    fn test_resolve_type_string_parent() {
        let mut scope = ScopeStack::new();
        scope.push_class(
            "App\\Child".to_string(),
            crate::symbol::scope::ClassKind::Class,
        );
        let mut type_db = TypeDatabase::new();
        type_db.add_uppers("App\\Child", vec!["App\\Base".to_string()]);
        let resolver = NameResolver::new();

        assert_eq!(
            resolve_type_string_to_fqn("parent", "App\\Child", &scope, &type_db, &resolver),
            Some("App\\Base".to_string())
        );
    }
}
