//! Reference handlers for PHP expressions.
//!
//! Produces SCIP reference occurrences for PHP symbol usages:
//! class names, method/function calls, property access, class constants,
//! and type hint references.

#![allow(unused_variables)]

use crate::indexing::context::IndexingContext;
use crate::parser::ast::PhpNode;
use crate::parser::cst::node_text;

/// Convert a tree-sitter Node to a SCIP range [start_line, start_col, end_line, end_col].
fn node_to_scip_range(node: tree_sitter::Node) -> Vec<u32> {
    let start = node.start_position();
    let end = node.end_position();
    vec![start.row as u32, start.column as u32, end.row as u32, end.column as u32]
}

/// PHP primitive/built-in type names that should NOT produce class references in type hints.
fn is_primitive_type(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "int" | "string" | "bool" | "float" | "void" | "null" | "array"
            | "object" | "mixed" | "never" | "iterable" | "callable"
            | "self" | "static" | "parent" | "true" | "false"
    )
}

/// Check if a scope text is self/static/parent (case-insensitive).
fn is_special_scope(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(lower.as_str(), "self" | "static" | "parent")
}

/// Resolve a scope text to a class FQN.
/// For self/static/parent, use the current class from the scope stack.
/// For regular class names, use the name resolver.
fn resolve_scope_class(scope_text: &str, ctx: &IndexingContext) -> Option<String> {
    if is_special_scope(scope_text) {
        ctx.scope.current_class().map(|s| s.to_string())
    } else {
        Some(ctx.resolver.resolve_class(scope_text))
    }
}

/// Handle an expression node that may produce reference occurrences.
pub fn handle_expression(node: &PhpNode, ctx: &mut IndexingContext) {
    match node {
        PhpNode::New(new_node) => {
            handle_new(new_node, ctx);
        }
        PhpNode::StaticCall(static_call) => {
            handle_static_call(static_call, ctx);
        }
        PhpNode::FuncCall(func_call) => {
            handle_func_call(func_call, ctx);
        }
        PhpNode::MethodCall(method_call) => {
            handle_method_call(method_call, ctx);
        }
        PhpNode::PropertyFetch(prop_fetch) => {
            handle_property_fetch(prop_fetch, ctx);
        }
        PhpNode::StaticPropertyFetch(static_prop) => {
            handle_static_property_fetch(static_prop, ctx);
        }
        PhpNode::ClassConstFetch(const_fetch) => {
            handle_class_const_fetch(const_fetch, ctx);
        }
        // Variable, Assign, Foreach, Name — no references emitted for now
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11.1: Class Name References
// ═══════════════════════════════════════════════════════════════════════════════

/// Handle `new Foo()` — emit class reference for `Foo`.
fn handle_new(node: &crate::parser::ast::NewNode, ctx: &mut IndexingContext) {
    let class_name = match node.class_name() {
        Some(name) => name,
        None => return, // dynamic: `new $className()`
    };

    // Skip self/static/parent — no class name reference for these
    if is_special_scope(class_name) {
        return;
    }

    let class_name_node = match node.class_name_node() {
        Some(n) => n,
        None => return,
    };

    let resolved = ctx.resolver.resolve_class(class_name);
    let pkg = &ctx.namer.project_package;
    let ver = &ctx.namer.project_version;
    let symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
    ctx.add_reference(symbol, node_to_scip_range(class_name_node));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11.2: Method and Function Call References
// ═══════════════════════════════════════════════════════════════════════════════

/// Handle `Foo::method()` — emit scope class reference + method reference.
fn handle_static_call(node: &crate::parser::ast::StaticCallNode, ctx: &mut IndexingContext) {
    let scope_text = match node.scope_text() {
        Some(s) => s,
        None => return,
    };
    let method_name = match node.method_name() {
        Some(n) => n,
        None => return, // dynamic method: `Foo::$method()`
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    // Resolve the class FQN
    let class_fqn = match resolve_scope_class(scope_text, ctx) {
        Some(fqn) => fqn,
        None => return, // self/static outside class context
    };

    // Emit class name reference (only for non-special scopes)
    if !is_special_scope(scope_text) {
        if let Some(scope_node) = node.scope() {
            let class_symbol = ctx.namer.symbol_for_class_like(&class_fqn, &pkg, &ver);
            ctx.add_reference(class_symbol, node_to_scip_range(scope_node));
        }
    }

    // Emit method reference
    if let Some(name_node) = node.method_name_node() {
        let method_symbol = ctx.namer.symbol_for_method(&class_fqn, method_name, &pkg, &ver);
        ctx.add_reference(method_symbol, node_to_scip_range(name_node));
    }
}

/// Handle `func_name()` — emit function reference.
fn handle_func_call(node: &crate::parser::ast::FuncCallNode, ctx: &mut IndexingContext) {
    let func_name = match node.function_name() {
        Some(name) => name,
        None => return, // dynamic call: `$func()`
    };
    let func_node = match node.function_node() {
        Some(n) => n,
        None => return,
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    let resolution = ctx.resolver.resolve_function(func_name);
    let fqn = resolution.primary().to_string();
    let symbol = ctx.namer.symbol_for_function(&fqn, &pkg, &ver);
    ctx.add_reference(symbol, node_to_scip_range(func_node));
}

/// Handle `$this->method()` — emit method reference for current class.
/// For unknown objects, skip (no panic).
fn handle_method_call(node: &crate::parser::ast::MethodCallNode, ctx: &mut IndexingContext) {
    let method_name = match node.method_name() {
        Some(n) => n,
        None => return, // dynamic method: `$obj->$method()`
    };

    let method_name_node = match node.method_name_node() {
        Some(n) => n,
        None => return,
    };

    // Check if the object is $this
    let object_node = match node.object() {
        Some(n) => n,
        None => return,
    };

    let object_text = node_text(object_node, ctx.source);
    if object_text == "$this" {
        // Resolve to current class
        let class_fqn = match ctx.scope.current_class() {
            Some(fqn) => fqn.to_string(),
            None => return,
        };
        let pkg = ctx.namer.project_package.clone();
        let ver = ctx.namer.project_version.clone();
        let method_symbol = ctx.namer.symbol_for_method(&class_fqn, method_name, &pkg, &ver);
        ctx.add_reference(method_symbol, node_to_scip_range(method_name_node));
    }
    // For other objects ($obj->method()), skip — no type info available
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11.3: Property and Constant Access References
// ═══════════════════════════════════════════════════════════════════════════════

/// Handle `$this->prop` — emit property reference for current class.
fn handle_property_fetch(node: &crate::parser::ast::PropertyFetchNode, ctx: &mut IndexingContext) {
    let prop_name = match node.property_name() {
        Some(n) => n,
        None => return, // dynamic property: `$obj->$prop`
    };

    let prop_name_node = match node.property_name_node() {
        Some(n) => n,
        None => return,
    };

    // Check if the object is $this
    let object_node = match node.object() {
        Some(n) => n,
        None => return,
    };

    let object_text = node_text(object_node, ctx.source);
    if object_text == "$this" {
        let class_fqn = match ctx.scope.current_class() {
            Some(fqn) => fqn.to_string(),
            None => return,
        };
        let pkg = ctx.namer.project_package.clone();
        let ver = ctx.namer.project_version.clone();
        let prop_symbol = ctx.namer.symbol_for_property(&class_fqn, prop_name, &pkg, &ver);
        ctx.add_reference(prop_symbol, node_to_scip_range(prop_name_node));
    }
    // For other objects, skip — no type info
}

/// Handle `Foo::$prop` — emit scope class reference + property reference.
fn handle_static_property_fetch(
    node: &crate::parser::ast::StaticPropertyFetchNode,
    ctx: &mut IndexingContext,
) {
    let scope_text = match node.scope_text() {
        Some(s) => s,
        None => return,
    };
    let prop_name_raw = match node.property_name() {
        Some(n) => n,
        None => return,
    };

    // Strip leading $ from property name — symbol_for_property expects name without $
    let prop_name = prop_name_raw.trim_start_matches('$');

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    let class_fqn = match resolve_scope_class(scope_text, ctx) {
        Some(fqn) => fqn,
        None => return,
    };

    // Emit class name reference (only for non-special scopes)
    if !is_special_scope(scope_text) {
        if let Some(scope_node) = node.scope() {
            let class_symbol = ctx.namer.symbol_for_class_like(&class_fqn, &pkg, &ver);
            ctx.add_reference(class_symbol, node_to_scip_range(scope_node));
        }
    }

    // Emit property reference
    if let Some(prop_node) = node.property_name_node() {
        let prop_symbol = ctx.namer.symbol_for_property(&class_fqn, prop_name, &pkg, &ver);
        ctx.add_reference(prop_symbol, node_to_scip_range(prop_node));
    }
}

/// Handle `Foo::CONST` — emit scope class reference + constant reference.
/// Skip `Foo::class` pseudo-constant.
fn handle_class_const_fetch(
    node: &crate::parser::ast::ClassConstFetchNode,
    ctx: &mut IndexingContext,
) {
    let scope_text = match node.scope_text() {
        Some(s) => s,
        None => return,
    };
    let const_name = match node.const_name() {
        Some(n) => n,
        None => return,
    };

    // Skip the `::class` pseudo-constant
    if const_name == "class" {
        return;
    }

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    let class_fqn = match resolve_scope_class(scope_text, ctx) {
        Some(fqn) => fqn,
        None => return,
    };

    // Emit class name reference (only for non-special scopes)
    if !is_special_scope(scope_text) {
        if let Some(scope_node) = node.scope() {
            let class_symbol = ctx.namer.symbol_for_class_like(&class_fqn, &pkg, &ver);
            ctx.add_reference(class_symbol, node_to_scip_range(scope_node));
        }
    }

    // Emit constant reference
    if let Some(name_node) = node.const_name_node() {
        let const_symbol = ctx.namer.symbol_for_class_const(&class_fqn, const_name, &pkg, &ver);
        ctx.add_reference(const_symbol, node_to_scip_range(name_node));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11.1 (continued): instanceof and catch clause
// ═══════════════════════════════════════════════════════════════════════════════

/// Handle `$x instanceof Foo` — emit reference for the class name `Foo`.
///
/// Called directly from the traversal driver for `binary_expression` nodes
/// where the operator is `instanceof`.
pub fn handle_instanceof(node: tree_sitter::Node, ctx: &mut IndexingContext) {
    // binary_expression has: left (expression), operator, right (class name)
    if let Some(right) = node.child_by_field_name("right") {
        // The right side may be a name, qualified_name, or namespace_name directly,
        // or it may be wrapped in a named_type
        let name_node = if matches!(right.kind(), "name" | "qualified_name" | "namespace_name") {
            Some(right)
        } else {
            // Check named children for a name node
            let mut found = None;
            for i in 0..right.named_child_count() {
                if let Some(child) = right.named_child(i) {
                    if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                        found = Some(child);
                        break;
                    }
                }
            }
            found
        };

        if let Some(nn) = name_node {
            let name = node_text(nn, ctx.source);
            if !is_special_scope(name) {
                let resolved = ctx.resolver.resolve_class(name);
                let pkg = &ctx.namer.project_package;
                let ver = &ctx.namer.project_version;
                let symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
                ctx.add_reference(symbol, node_to_scip_range(nn));
            }
        }
    }
}

/// Handle `catch (Exception $e)` — emit reference for each exception class name.
///
/// Called directly from the traversal driver for `catch_clause` nodes.
pub fn handle_catch_clause(node: tree_sitter::Node, ctx: &mut IndexingContext) {
    // catch_clause contains: type (the exception type) and body
    // The type can be a single name, or a union (Exception | RuntimeException)
    if let Some(type_node) = node.child_by_field_name("type") {
        emit_catch_type_references(type_node, ctx);
    }
}

/// Emit references for exception types in a catch clause.
/// Handles both single types and union types (Exception | RuntimeException).
fn emit_catch_type_references(type_node: tree_sitter::Node, ctx: &mut IndexingContext) {
    match type_node.kind() {
        "named_type" | "name" | "qualified_name" | "namespace_name" => {
            let name_node = if type_node.kind() == "named_type" {
                // named_type wraps a name/qualified_name child
                type_node.named_child(0)
            } else {
                Some(type_node)
            };
            if let Some(name_n) = name_node {
                if matches!(name_n.kind(), "name" | "qualified_name" | "namespace_name") {
                    let name = node_text(name_n, ctx.source);
                    let resolved = ctx.resolver.resolve_class(name);
                    let pkg = &ctx.namer.project_package;
                    let ver = &ctx.namer.project_version;
                    let symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
                    ctx.add_reference(symbol, node_to_scip_range(name_n));
                }
            }
        }
        "union_type" | "type_list" => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_catch_type_references(child, ctx);
                }
            }
        }
        _ => {
            // Fallback: try children
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_catch_type_references(child, ctx);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11.4: Type Hint References
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit references for class names appearing in type hints.
///
/// This is called from the traversal driver (or definition emitters) for type
/// nodes found in parameter types, return types, and property types.
///
/// Recursively handles: named_type, optional_type (?Foo), union_type (Foo|Bar),
/// intersection_type (Foo&Bar), disjunctive_normal_form_type ((Foo&Bar)|null).
pub fn emit_type_hint_references(type_node: tree_sitter::Node, ctx: &mut IndexingContext) {
    match type_node.kind() {
        // A named type like `Foo` or `App\Models\User`
        "named_type" => {
            // The child is usually a "name" or "qualified_name" node
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                        let name = node_text(child, ctx.source);
                        if !is_primitive_type(name) {
                            let resolved = ctx.resolver.resolve_class(name);
                            let pkg = &ctx.namer.project_package;
                            let ver = &ctx.namer.project_version;
                            let symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
                            ctx.add_reference(symbol, node_to_scip_range(child));
                        }
                    }
                }
            }
        }

        // A simple name node used as a type (e.g., `Foo` in some contexts)
        "name" | "qualified_name" | "namespace_name" => {
            let name = node_text(type_node, ctx.source);
            if !is_primitive_type(name) {
                let resolved = ctx.resolver.resolve_class(name);
                let pkg = &ctx.namer.project_package;
                let ver = &ctx.namer.project_version;
                let symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
                ctx.add_reference(symbol, node_to_scip_range(type_node));
            }
        }

        // ?Foo — nullable type
        "optional_type" => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_type_hint_references(child, ctx);
                }
            }
        }

        // Foo|Bar — union type
        "union_type" => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_type_hint_references(child, ctx);
                }
            }
        }

        // Foo&Bar — intersection type
        "intersection_type" => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_type_hint_references(child, ctx);
                }
            }
        }

        // (Foo&Bar)|null — DNF type
        "disjunctive_normal_form_type" => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_type_hint_references(child, ctx);
                }
            }
        }

        // Primitive types: int, string, etc. — skip
        "primitive_type" => {}

        // Anything else — try recursing into children
        _ => {
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    emit_type_hint_references(child, ctx);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use crate::composer::Composer;
    use crate::indexing::{index_file, FileResult, IndexingContext};
    use crate::output::scip::symbol_roles;
    use crate::parser::PhpParser;
    use crate::symbol::namer::SymbolNamer;
    use crate::types::TypeDatabase;

    fn setup_and_index(php_source: &str) -> FileResult {
        let mut parser = PhpParser::new();
        let parsed = parser.parse(php_source, "test.php").unwrap();

        let type_db = TypeDatabase::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let file_path = dir.path().join("test.php");

        let mut ctx = IndexingContext::new(
            &file_path,
            &parsed.source,
            &type_db,
            &composer,
            &namer,
            dir.path(),
        );

        index_file(&parsed, &mut ctx);
        ctx.into_result()
    }

    /// Helper: find all reference occurrences matching a symbol substring.
    fn find_refs<'a>(
        result: &'a FileResult,
        symbol_substr: &str,
    ) -> Vec<&'a crate::output::scip::Occurrence> {
        result
            .occurrences
            .iter()
            .filter(|o| {
                o.symbol.contains(symbol_substr) && o.symbol_roles == symbol_roles::REFERENCE
            })
            .collect()
    }

    /// Helper: find all reference occurrences matching a symbol substring on a specific line.
    fn find_refs_on_line<'a>(
        result: &'a FileResult,
        symbol_substr: &str,
        line: u32,
    ) -> Vec<&'a crate::output::scip::Occurrence> {
        result
            .occurrences
            .iter()
            .filter(|o| {
                o.symbol.contains(symbol_substr)
                    && o.symbol_roles == symbol_roles::REFERENCE
                    && o.range[0] == line
            })
            .collect()
    }

    // ── 11.1: Class Name References ────────────────────────────────────────

    #[test]
    fn test_new_class_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class Factory {
    public function create(): void {
        $u = new User();
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs(&result, "User#");
        // At least one reference for `new User()`
        assert!(
            !refs.is_empty(),
            "Expected a reference for User in `new User()`"
        );
        // The reference should be on line 7 (0-indexed)
        let line7_refs = find_refs_on_line(&result, "User#", 7);
        assert!(
            !line7_refs.is_empty(),
            "Expected User reference on line 7"
        );
    }

    #[test]
    fn test_new_self_no_reference() {
        let source = r#"<?php
namespace App;

class Foo {
    public static function create(): void {
        return new self();
    }
}
"#;
        let result = setup_and_index(source);
        // No class name reference should be emitted for `new self()`
        let refs = find_refs_on_line(&result, "Foo#", 5);
        // The `new self()` line should NOT produce a class reference occurrence on that line
        assert!(
            refs.is_empty(),
            "Expected no class reference for `new self()` on line 5"
        );
    }

    #[test]
    fn test_new_qualified_class() {
        let source = r#"<?php
namespace App;

class Factory {
    public function create(): void {
        $u = new \DateTime();
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "DateTime#", 5);
        assert!(
            !refs.is_empty(),
            "Expected a reference for \\DateTime"
        );
    }

    // ── 11.2: Static Call References ───────────────────────────────────────

    #[test]
    fn test_static_call_class_and_method_reference() {
        let source = r#"<?php
namespace App;

use App\Services\UserService;

class Controller {
    public function handle(): void {
        UserService::findAll();
    }
}
"#;
        let result = setup_and_index(source);

        // Class reference for UserService
        let class_refs = find_refs_on_line(&result, "UserService#", 7);
        assert!(
            !class_refs.is_empty(),
            "Expected a class reference for UserService"
        );
        // Method reference for findAll
        let method_refs = find_refs_on_line(&result, "findAll().", 7);
        assert!(
            !method_refs.is_empty(),
            "Expected a method reference for findAll"
        );
    }

    #[test]
    fn test_static_call_self_no_class_ref() {
        let source = r#"<?php
namespace App;

class Foo {
    public static function bar(): void {}
    public static function baz(): void {
        self::bar();
    }
}
"#;
        let result = setup_and_index(source);

        // No class reference for `self::` on line 6
        let class_refs = find_refs_on_line(&result, "Foo#", 6);
        // Filter out any that contain method descriptors
        let pure_class_refs: Vec<_> = class_refs
            .iter()
            .filter(|o| !o.symbol.contains("()."))
            .collect();
        assert!(
            pure_class_refs.is_empty(),
            "Expected no class reference for `self::`"
        );

        // But method reference should be emitted for `bar()`
        let method_refs = find_refs_on_line(&result, "bar().", 6);
        assert!(
            !method_refs.is_empty(),
            "Expected method reference for self::bar()"
        );
    }

    // ── 11.2: Function Call References ─────────────────────────────────────

    #[test]
    fn test_function_call_reference() {
        let source = r#"<?php
namespace App;

use function App\Helpers\format_date;

class Foo {
    public function bar(): void {
        format_date('2024-01-01');
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "format_date().", 7);
        assert!(
            !refs.is_empty(),
            "Expected function reference for format_date()"
        );
    }

    #[test]
    fn test_global_function_call() {
        let source = r#"<?php
$len = strlen('hello');
"#;
        let result = setup_and_index(source);
        let refs = find_refs(&result, "strlen().");
        assert!(
            !refs.is_empty(),
            "Expected function reference for strlen()"
        );
    }

    // ── 11.2: Method Call References ───────────────────────────────────────

    #[test]
    fn test_this_method_call_reference() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {}
    public function baz(): void {
        $this->bar();
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "bar().", 6);
        assert!(
            !refs.is_empty(),
            "Expected method reference for $this->bar()"
        );
    }

    #[test]
    fn test_unknown_object_method_call_skipped() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar($obj): void {
        $obj->someMethod();
    }
}
"#;
        let result = setup_and_index(source);
        // No reference should be emitted for unknown object method calls
        let refs = find_refs(&result, "someMethod().");
        assert!(
            refs.is_empty(),
            "Expected no method reference for $obj->someMethod()"
        );
    }

    // ── 11.3: Property Fetch References ────────────────────────────────────

    #[test]
    fn test_this_property_reference() {
        let source = r#"<?php
namespace App;

class Foo {
    private string $name;
    public function getName(): string {
        return $this->name;
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "$name.", 6);
        assert!(
            !refs.is_empty(),
            "Expected property reference for $this->name"
        );
    }

    #[test]
    fn test_unknown_object_property_skipped() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar($obj): void {
        $val = $obj->prop;
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs(&result, "$prop.");
        assert!(
            refs.is_empty(),
            "Expected no property reference for $obj->prop"
        );
    }

    // ── 11.3: Static Property References ───────────────────────────────────

    #[test]
    fn test_static_property_reference() {
        let source = r#"<?php
namespace App;

class Config {
    public static int $count = 0;
    public static function inc(): void {
        self::$count++;
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "$count.", 6);
        assert!(
            !refs.is_empty(),
            "Expected property reference for self::$count"
        );
    }

    #[test]
    fn test_static_property_with_class_name() {
        let source = r#"<?php
namespace App;

use App\Config;

class Foo {
    public function bar(): void {
        Config::$instance;
    }
}
"#;
        let result = setup_and_index(source);
        // Class reference for Config
        let class_refs = find_refs_on_line(&result, "Config#", 7);
        let pure_class_refs: Vec<_> = class_refs
            .iter()
            .filter(|o| !o.symbol.contains("$"))
            .collect();
        assert!(
            !pure_class_refs.is_empty(),
            "Expected class reference for Config"
        );
        // Property reference
        let prop_refs = find_refs_on_line(&result, "$instance.", 7);
        assert!(
            !prop_refs.is_empty(),
            "Expected property reference for Config::$instance"
        );
    }

    // ── 11.3: Class Constant References ────────────────────────────────────

    #[test]
    fn test_class_const_reference() {
        let source = r#"<?php
namespace App;

class Config {
    const VERSION = '1.0';
    public function getVersion(): string {
        return self::VERSION;
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "VERSION.", 6);
        assert!(
            !refs.is_empty(),
            "Expected constant reference for self::VERSION"
        );
    }

    #[test]
    fn test_class_const_with_class_name() {
        let source = r#"<?php
namespace App;

use App\Enums\Status;

class Foo {
    public function bar(): void {
        $s = Status::Active;
    }
}
"#;
        let result = setup_and_index(source);
        // Class reference for Status
        let class_refs = find_refs_on_line(&result, "Status#", 7);
        let pure_class_refs: Vec<_> = class_refs
            .iter()
            .filter(|o| !o.symbol.contains("Active"))
            .collect();
        assert!(
            !pure_class_refs.is_empty(),
            "Expected class reference for Status"
        );
        // Constant reference for Active
        let const_refs = find_refs_on_line(&result, "Active.", 7);
        assert!(
            !const_refs.is_empty(),
            "Expected constant reference for Status::Active"
        );
    }

    #[test]
    fn test_class_pseudo_const_skipped() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {
        $c = Foo::class;
    }
}
"#;
        let result = setup_and_index(source);
        // The `::class` pseudo-constant should NOT produce a constant reference
        let refs: Vec<_> = result
            .occurrences
            .iter()
            .filter(|o| {
                o.symbol_roles == symbol_roles::REFERENCE
                    && o.range[0] == 5
                    && o.symbol.contains("class.")
            })
            .collect();
        assert!(
            refs.is_empty(),
            "Expected no constant reference for ::class pseudo-constant"
        );
    }

    // ── 11.4: Type Hint References ─────────────────────────────────────────

    #[test]
    fn test_param_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class UserService {
    public function find(User $user): void {}
}
"#;
        let result = setup_and_index(source);
        // There should be a reference for User in the parameter type hint
        let refs = find_refs_on_line(&result, "User#", 6);
        assert!(
            !refs.is_empty(),
            "Expected type hint reference for User parameter type"
        );
    }

    #[test]
    fn test_return_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class UserService {
    public function find(): User {}
}
"#;
        let result = setup_and_index(source);
        // There should be a reference for User in the return type
        let refs = find_refs_on_line(&result, "User#", 6);
        assert!(
            !refs.is_empty(),
            "Expected type hint reference for User return type"
        );
    }

    #[test]
    fn test_nullable_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class UserService {
    public function find(): ?User {}
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "User#", 6);
        assert!(
            !refs.is_empty(),
            "Expected type hint reference for ?User return type"
        );
    }

    #[test]
    fn test_union_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;
use App\Models\Admin;

class UserService {
    public function find(): User|Admin {}
}
"#;
        let result = setup_and_index(source);
        let user_refs = find_refs_on_line(&result, "User#", 7);
        assert!(
            !user_refs.is_empty(),
            "Expected type hint reference for User in union type"
        );
        let admin_refs = find_refs_on_line(&result, "Admin#", 7);
        assert!(
            !admin_refs.is_empty(),
            "Expected type hint reference for Admin in union type"
        );
    }

    #[test]
    fn test_primitive_type_hint_no_reference() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(string $name, int $age): void {}
}
"#;
        let result = setup_and_index(source);
        // No references should be emitted for primitive types
        let string_refs: Vec<_> = result
            .occurrences
            .iter()
            .filter(|o| {
                o.symbol_roles == symbol_roles::REFERENCE && o.symbol.contains("string#")
            })
            .collect();
        assert!(
            string_refs.is_empty(),
            "Expected no reference for primitive type 'string'"
        );
    }

    #[test]
    fn test_property_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class UserService {
    private User $user;
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "User#", 6);
        assert!(
            !refs.is_empty(),
            "Expected type hint reference for User property type"
        );
    }

    // ── 11.1 (continued): instanceof and catch ────────────────────────────

    #[test]
    fn test_instanceof_reference() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class Checker {
    public function check($obj): void {
        if ($obj instanceof User) {
            // ...
        }
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs_on_line(&result, "User#", 7);
        assert!(
            !refs.is_empty(),
            "Expected class reference for instanceof User"
        );
    }

    #[test]
    fn test_catch_clause_reference() {
        let source = r#"<?php
namespace App;

class Handler {
    public function handle(): void {
        try {
            // ...
        } catch (\RuntimeException $e) {
            // ...
        }
    }
}
"#;
        let result = setup_and_index(source);
        let refs = find_refs(&result, "RuntimeException#");
        assert!(
            !refs.is_empty(),
            "Expected class reference for catch (RuntimeException)"
        );
    }

    // ── Intersection type hint ────────────────────────────────────────────

    #[test]
    fn test_intersection_type_hint_reference() {
        let source = r#"<?php
namespace App;

use App\Contracts\Cacheable;
use App\Contracts\Loggable;

class Foo {
    public function process(Cacheable&Loggable $obj): void {}
}
"#;
        let result = setup_and_index(source);
        let cacheable_refs = find_refs_on_line(&result, "Cacheable#", 7);
        assert!(
            !cacheable_refs.is_empty(),
            "Expected type hint reference for Cacheable in intersection type"
        );
        let loggable_refs = find_refs_on_line(&result, "Loggable#", 7);
        assert!(
            !loggable_refs.is_empty(),
            "Expected type hint reference for Loggable in intersection type"
        );
    }

    // ── Multiple references in one file ──────────────────────────────────

    #[test]
    fn test_multiple_references_comprehensive() {
        let source = r#"<?php
namespace App;

use App\Models\User;
use App\Services\Logger;

class UserController {
    private User $user;
    private Logger $logger;

    public function __construct(User $user, Logger $logger) {
        $this->user = $user;
        $this->logger = $logger;
    }

    public function find(): User {
        $this->logger->info('finding');
        return $this->user;
    }
}
"#;
        let result = setup_and_index(source);

        // Check that User has references (property type, param type, return type, $this->user)
        let user_refs = find_refs(&result, "User#");
        assert!(
            user_refs.len() >= 3,
            "Expected at least 3 references for User, got {}",
            user_refs.len()
        );

        // Check Logger has references (property type, param type)
        let logger_refs = find_refs(&result, "Logger#");
        assert!(
            logger_refs.len() >= 2,
            "Expected at least 2 references for Logger, got {}",
            logger_refs.len()
        );

        // Check $this->user property references
        let user_prop_refs = find_refs(&result, "$user.");
        assert!(
            !user_prop_refs.is_empty(),
            "Expected property references for $this->user"
        );
    }
}
