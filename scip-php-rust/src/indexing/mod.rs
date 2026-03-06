//! SCIP occurrence and symbol emission (Pass 2 indexing).
//!
//! Contains the CST traversal driver (`index_file`), per-file indexing context,
//! and stub emitters for definitions and references.

pub mod calls;
pub mod context;
pub mod definitions;
pub mod expression_tracker;
pub mod locals;
pub mod references;

// Re-exports
pub use context::{FileResult, IndexingContext};

use crate::indexing::locals::ScopeKind;
use crate::names::resolver::NameResolver;
use crate::parser::ast::{classify_node, PhpNode};
use crate::parser::cst::{child_by_kind, find_all, node_text};
use crate::parser::ParsedFile;
use crate::symbol::scope::ClassKind as ScopeClassKind;

/// Index a parsed PHP file: walk the CST, classify nodes, push/pop scopes,
/// and invoke definition/reference emitters.
///
/// The context must already have its `NameResolver` initialized (namespace and
/// use declarations set up) before calling this function, OR this function
/// will do the initialization itself from the CST.
pub fn index_file(parsed: &ParsedFile, ctx: &mut IndexingContext) {
    let root = parsed.root();
    let source = &parsed.source;

    // Pre-pass: initialize the name resolver from namespace and use declarations.
    initialize_resolver(&mut ctx.resolver, root, source);

    // Walk the CST using a TreeCursor for efficient depth-first traversal.
    let mut cursor = root.walk();
    let mut scope_depth: u32 = 0;
    // Stack to track which nodes pushed a scope, so we pop on exit.
    let mut scope_pushed_stack: Vec<bool> = Vec::new();
    // Parallel stack: tracks whether a local variable scope was pushed for each node.
    // Values: 0 = no local scope, 1 = normal scope pushed, 2 = arrow function (no-op)
    let mut local_scope_stack: Vec<u8> = Vec::new();

    let mut did_enter = true;

    loop {
        if did_enter {
            let node = cursor.node();

            // Enter: classify and potentially push scope
            let (pushed, local_kind) = enter_node(node, source, ctx);
            scope_pushed_stack.push(pushed);
            local_scope_stack.push(local_kind);
            if pushed {
                scope_depth += 1;
            }

            // Try to descend into children
            if cursor.goto_first_child() {
                continue;
            }
        }

        // Exit the current node (pop scope if it was pushed)
        if let Some(pushed) = scope_pushed_stack.pop() {
            if pushed {
                ctx.scope.pop();
                scope_depth -= 1;
            }
        }
        if let Some(local_kind) = local_scope_stack.pop() {
            match local_kind {
                1 => ctx.local_tracker.exit_scope(),
                2 => ctx.local_tracker.exit_arrow_function(),
                _ => {}
            }
        }

        // Try the next sibling
        if cursor.goto_next_sibling() {
            did_enter = true;
            continue;
        }

        // No sibling — go up to parent
        if cursor.goto_parent() {
            did_enter = false;
            continue;
        }

        // Reached the root — done
        break;
    }

    debug_assert_eq!(
        scope_depth, 0,
        "scope depth should return to 0 after traversal, got {}",
        scope_depth
    );
}

/// Initialize the name resolver by scanning for namespace and use declarations.
fn initialize_resolver(resolver: &mut NameResolver, root: tree_sitter::Node, source: &[u8]) {
    // Find and process namespace definitions
    let mut ns_nodes = Vec::new();
    find_all(root, "namespace_definition", &mut ns_nodes);
    for ns_node in &ns_nodes {
        if let Some(ns_name) = child_by_kind(*ns_node, "namespace_name") {
            resolver.enter_namespace(node_text(ns_name, source));
        }
    }

    // Find and process use declarations
    let mut use_nodes = Vec::new();
    find_all(root, "namespace_use_declaration", &mut use_nodes);
    for use_node in &use_nodes {
        let imports = NameResolver::parse_use_declaration(*use_node, source);
        for import in imports {
            resolver.add_import(import);
        }
    }
}

/// Populate var_types from a formal_parameters node.
/// Iterates parameter children, resolves type hints to FQNs, skips primitives.
/// Falls back to PHPDoc @param annotations when native type hints are absent.
fn populate_var_types_from_params(
    params_node: Option<tree_sitter::Node>,
    decl_node: tree_sitter::Node,
    ctx: &mut IndexingContext,
) {
    let params_node = match params_node {
        Some(n) => n,
        None => return,
    };

    // Parse PHPDoc once for fallback
    let phpdoc = crate::types::phpdoc::get_docblock(decl_node, ctx.source)
        .map(|doc| crate::types::phpdoc::parse_phpdoc(&doc));

    // Iterate named children — each is a simple_parameter, variadic_parameter, or property_promotion_parameter
    for i in 0..params_node.named_child_count() {
        let child = match params_node.named_child(i) {
            Some(c) => c,
            None => continue,
        };

        // Get the parameter name
        let param_name = match child.child_by_field_name("name") {
            Some(n) => node_text(n, ctx.source),
            None => continue,
        };

        // Try native type hint first
        let native_fqn = child.child_by_field_name("type").map(|type_node| {
            crate::types::resolver::resolve_type_node_to_fqn(
                type_node,
                ctx.source,
                &ctx.resolver,
            )
        });

        let has_native = native_fqn.as_ref().map_or(false, |fqn| !fqn.is_empty());

        if has_native {
            ctx.var_types.set(param_name, native_fqn.unwrap());
            continue;
        }

        // Fall back to PHPDoc @param
        let param_name_without_dollar = param_name.trim_start_matches('$');
        if let Some(ref doc) = phpdoc {
            // DocInfo.params is Vec<(type_expr, param_name_without_dollar)>
            if let Some((type_expr, _)) = doc.params.iter().find(|(_, name)| name == param_name_without_dollar) {
                if let Some(fqn) = resolve_phpdoc_type_to_fqn(type_expr, ctx) {
                    ctx.var_types.set(param_name, fqn);
                }
            }
        }
    }
}

/// Resolve a PHPDoc type expression to a class FQN.
/// Strips nullable, takes first union member, strips generics.
fn resolve_phpdoc_type_to_fqn(type_expr: &str, ctx: &IndexingContext) -> Option<String> {
    let class_name = crate::types::phpdoc::normalize_type(type_expr)?;
    let fqn = ctx.resolver.resolve_class(&class_name);
    if fqn.is_empty() {
        None
    } else {
        Some(fqn)
    }
}

/// Process a node on entry during traversal.
///
/// Returns `(scope_pushed, local_scope_kind)` where:
/// - `scope_pushed`: true if a ScopeStack frame was pushed (must be popped on exit)
/// - `local_scope_kind`: 0 = no local scope, 1 = normal scope pushed, 2 = arrow function (no-op)
fn enter_node(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) -> (bool, u8) {
    // Skip error nodes to avoid panics on malformed input
    if node.has_error() && node.kind() == "ERROR" {
        return (false, 0);
    }

    let php_node = classify_node(node, source);

    match &php_node {
        // --- Scope-defining nodes: push scope frames ---
        PhpNode::Namespace(ns) => {
            let ns_name = ns.name();
            ctx.scope.push_namespace(ns_name.to_string());
            return (true, 0);
        }

        PhpNode::ClassLike(class_node) => {
            let class_name = class_node.name().unwrap_or("");
            // Build FQN from resolver namespace (handles both bracketed and semicolon namespaces)
            let ns = ctx.resolver.namespace();
            let fqn = if ns.is_empty() {
                class_name.to_string()
            } else {
                format!("{}\\{}", ns, class_name)
            };

            // Convert ast::ClassKind to scope::ClassKind
            let scope_kind = match class_node.class_kind() {
                crate::parser::ast::ClassKind::Class => ScopeClassKind::Class,
                crate::parser::ast::ClassKind::Interface => ScopeClassKind::Interface,
                crate::parser::ast::ClassKind::Trait => ScopeClassKind::Trait,
                crate::parser::ast::ClassKind::Enum => ScopeClassKind::Enum,
            };

            ctx.scope.push_class(fqn, scope_kind);

            // Call stub definition emitter
            definitions::emit_class_definition(class_node, ctx);
            return (true, 0);
        }

        PhpNode::Method(method_node) => {
            let name = method_node.name().to_string();
            let is_static = method_node.is_static();
            ctx.scope.push_method(name, is_static);

            // Push local variable scope
            ctx.local_tracker.enter_scope(ScopeKind::Function);

            // Clear and populate var_types from parameter type hints + PHPDoc fallback
            ctx.var_types.clear();
            populate_var_types_from_params(method_node.parameters(), method_node.node, ctx);

            // Call stub definition emitter
            definitions::emit_method_definition(method_node, ctx);
            return (true, 1);
        }

        PhpNode::Function(func_node) => {
            let name = func_node.name();
            // Build FQN from resolver namespace (handles both bracketed and semicolon namespaces)
            let ns = ctx.resolver.namespace();
            let fqn = if ns.is_empty() {
                name.to_string()
            } else {
                format!("{}\\{}", ns, name)
            };
            ctx.scope.push_function(fqn);

            // Push local variable scope
            ctx.local_tracker.enter_scope(ScopeKind::Function);

            // Clear and populate var_types from parameter type hints + PHPDoc fallback
            ctx.var_types.clear();
            populate_var_types_from_params(func_node.parameters(), func_node.node, ctx);

            // Call stub definition emitter
            definitions::emit_function_definition(func_node, ctx);
            return (true, 1);
        }

        PhpNode::Closure(ref closure_node) => {
            // Save current class before pushing closure scope
            let enclosing_class = ctx.scope.current_class().map(|s| s.to_string());
            let is_static_closure = closure_node.is_static();

            ctx.scope.push_closure();

            // Push local variable scope
            ctx.local_tracker.enter_scope(ScopeKind::Closure);

            // Non-static closures inherit $this from the enclosing class
            if !is_static_closure {
                if let Some(class_fqn) = enclosing_class {
                    ctx.var_types.set("this", class_fqn);
                }
            }
            return (true, 1);
        }

        PhpNode::ArrowFunction(_) => {
            ctx.scope.push_arrow_function();
            ctx.local_tracker.enter_arrow_function();
            return (true, 2);
        }

        // --- Definition nodes: call stub emitters ---
        PhpNode::Param(param_node) => {
            definitions::emit_param_definition(param_node, ctx);
            // Register param in local tracker (the definition occurrence was already
            // emitted by emit_param_definition; we just need to record it so that
            // subsequent references resolve correctly).
            let param_name = param_node.name();
            // The local_id was allocated by emit_param_definition via next_local_id().
            // It's local_counter - 1 since it was just incremented.
            if ctx.local_counter > 0 {
                ctx.local_tracker.register_param(param_name, ctx.local_counter - 1);
            }
        }

        PhpNode::Property(prop_node) => {
            definitions::emit_property_definition(prop_node, ctx);
        }

        PhpNode::ClassConst(const_node) => {
            definitions::emit_class_const_definition(const_node, ctx);
        }

        PhpNode::EnumCase(case_node) => {
            definitions::emit_enum_case_definition(case_node, ctx);
        }

        // --- Assignment nodes: track variable types + handle sub-expressions ---
        PhpNode::Assign(ref assign_node) => {
            // Track variable type from assignment RHS
            if let Some(left) = assign_node.left() {
                if left.kind() == "variable_name" {
                    let var_name = node_text(left, source);
                    if let Some(right) = assign_node.right() {
                        if let Some(rhs_type) = crate::types::resolver::resolve_expr_type(
                            right,
                            source,
                            &ctx.var_types,
                            &ctx.scope,
                            ctx.type_db,
                            &ctx.resolver,
                        ) {
                            ctx.var_types.set(var_name, rhs_type);
                        }
                    }
                }

                // Define LHS variable(s) in local tracker
                detect_assignment_lhs(left, source, ctx);
            }
            references::handle_expression(&php_node, ctx);
        }

        // --- Expression/reference nodes: call stub handler + expression tracking ---
        PhpNode::MethodCall(ref method_call) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_method_call(
                method_call,
                source,
                &ctx.scope,
                &ctx.var_types,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::StaticCall(ref static_call) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_static_call(
                static_call,
                source,
                &ctx.scope,
                &ctx.var_types,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::FuncCall(ref func_call) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_func_call(
                func_call,
                source,
                &ctx.scope,
                &ctx.var_types,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::New(ref new_node) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_new_call(
                new_node,
                source,
                &ctx.scope,
                &ctx.var_types,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::PropertyFetch(ref prop_fetch) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_property_access(
                prop_fetch,
                source,
                &ctx.scope,
                &ctx.var_types,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::StaticPropertyFetch(ref static_prop) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_static_property_access(
                static_prop,
                source,
                &ctx.scope,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::ClassConstFetch(ref const_fetch) => {
            references::handle_expression(&php_node, ctx);
            let pkg = ctx.namer.project_package.clone();
            let ver = ctx.namer.project_version.clone();
            let rel = ctx.relative_path.clone();
            ctx.expression_tracker.track_class_const_access(
                const_fetch,
                source,
                &ctx.scope,
                ctx.type_db,
                &ctx.resolver,
                &rel,
                &pkg,
                &ver,
                ctx.namer,
            );
        }
        PhpNode::Variable(_)
        | PhpNode::Name(_) => {
            references::handle_expression(&php_node, ctx);
        }

        PhpNode::Foreach(ref foreach_node) => {
            references::handle_expression(&php_node, ctx);
            // Define foreach key and value variables
            detect_foreach_variables(foreach_node, source, ctx);
        }

        // Everything else — check for trait use declarations, instanceof, catch,
        // global declarations, static variable declarations
        _ => {
            // Handle `use Trait1, Trait2;` inside class bodies (use_declaration node)
            if node.kind() == "use_declaration" && ctx.scope.in_class_body() {
                definitions::emit_trait_use(node, ctx);
            }

            // Handle `$x instanceof Foo` — binary_expression with operator "instanceof"
            if node.kind() == "binary_expression" {
                if let Some(op) = node.child_by_field_name("operator") {
                    let op_text = node_text(op, source);
                    if op_text == "instanceof" {
                        references::handle_instanceof(node, ctx);
                    }
                }
            }

            // Handle `catch (Exception $e)` — emit reference for exception class + define variable
            if node.kind() == "catch_clause" {
                references::handle_catch_clause(node, ctx);
                detect_catch_variable(node, source, ctx);
            }

            // Handle `global $x, $y;`
            if node.kind() == "global_declaration" {
                detect_global_variables(node, source, ctx);
            }

            // Handle `static $x = 0, $y;`
            if node.kind() == "static_variable_declaration" {
                detect_static_variables(node, source, ctx);
            }
        }
    }

    (false, 0)
}

/// Detect variable definitions on the left-hand side of an assignment.
fn detect_assignment_lhs(lhs: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) {
    match lhs.kind() {
        "variable_name" => {
            // Skip variable variables ($$var): variable_name containing variable_name
            if lhs.named_child_count() > 0 {
                if let Some(child) = lhs.named_child(0) {
                    if child.kind() == "variable_name" {
                        return;
                    }
                }
            }
            let name = node_text(lhs, source).trim_start_matches('$');
            let range = locals::node_to_scip_range(lhs);
            ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
        }
        "list_literal" | "array_creation_expression" => {
            locals::define_destructuring_variables(
                lhs,
                source,
                &mut ctx.local_tracker,
                &mut ctx.local_counter,
            );
        }
        _ => {} // property access, array access, etc. — not local variable definitions
    }
}

/// Detect foreach key and value variable definitions.
///
/// tree-sitter-php foreach_statement CST structure:
///   foreach ( $expr as $value ) { ... }
///     -> named children: variable_name($expr), variable_name($value), compound_statement
///   foreach ( $expr as $key => $value ) { ... }
///     -> named children: variable_name($expr), pair($key => $value), compound_statement
fn detect_foreach_variables(
    foreach_node: &crate::parser::ast::ForeachNode,
    source: &[u8],
    ctx: &mut IndexingContext,
) {
    let node = foreach_node.node;

    // Walk named children to find the foreach value variable(s).
    // Skip the first named child (the iterable expression).
    // The second named child is either:
    //   - variable_name (simple: foreach ($items as $value))
    //   - pair (key-value: foreach ($items as $key => $value))
    //   - by_ref (reference: foreach ($items as &$value))
    //   - list_literal/array_creation_expression (destructuring)
    for i in 1..node.named_child_count() {
        let child = match node.named_child(i) {
            Some(c) => c,
            None => continue,
        };

        match child.kind() {
            "pair" => {
                // Key => Value: named children are [key_var, value_var]
                // pair children: <key> "=>" <value>
                if let Some(key_node) = child.named_child(0) {
                    define_foreach_var(key_node, source, ctx);
                }
                if let Some(value_node) = child.named_child(1) {
                    define_foreach_var(value_node, source, ctx);
                }
                return;
            }
            "variable_name" => {
                let name = node_text(child, source).trim_start_matches('$');
                let range = locals::node_to_scip_range(child);
                ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
                return;
            }
            "by_ref" => {
                // foreach ($items as &$value)
                for j in 0..child.named_child_count() {
                    if let Some(var_node) = child.named_child(j) {
                        if var_node.kind() == "variable_name" {
                            let name = node_text(var_node, source).trim_start_matches('$');
                            let range = locals::node_to_scip_range(var_node);
                            ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
                        }
                    }
                }
                return;
            }
            "list_literal" | "array_creation_expression" => {
                locals::define_destructuring_variables(
                    child,
                    source,
                    &mut ctx.local_tracker,
                    &mut ctx.local_counter,
                );
                return;
            }
            "compound_statement" => {
                // Body — stop looking
                return;
            }
            _ => {}
        }
    }
}

/// Helper to define a foreach key or value variable from a node that may be
/// a variable_name, by_ref, or destructuring expression.
fn define_foreach_var(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) {
    match node.kind() {
        "variable_name" => {
            let name = node_text(node, source).trim_start_matches('$');
            let range = locals::node_to_scip_range(node);
            ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
        }
        "by_ref" => {
            for i in 0..node.named_child_count() {
                if let Some(var_node) = node.named_child(i) {
                    if var_node.kind() == "variable_name" {
                        let name = node_text(var_node, source).trim_start_matches('$');
                        let range = locals::node_to_scip_range(var_node);
                        ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
                    }
                }
            }
        }
        "list_literal" | "array_creation_expression" => {
            locals::define_destructuring_variables(
                node,
                source,
                &mut ctx.local_tracker,
                &mut ctx.local_counter,
            );
        }
        _ => {}
    }
}

/// Detect catch clause exception variable definition.
fn detect_catch_variable(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) {
    // catch_clause: named children include the type(s) and optionally a variable_name
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == "variable_name" {
                let name = node_text(child, source).trim_start_matches('$');
                let range = locals::node_to_scip_range(child);
                ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
                break;
            }
        }
    }
}

/// Detect global declaration variable definitions: `global $x, $y;`
fn detect_global_variables(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if child.kind() == "variable_name" {
                let name = node_text(child, source).trim_start_matches('$');
                let range = locals::node_to_scip_range(child);
                ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
            }
        }
    }
}

/// Detect static variable declaration definitions: `static $x = 0, $y;`
fn detect_static_variables(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            // Each child is a static_variable_declaration element
            // Look for variable_name within
            if child.kind() == "variable_name" {
                let name = node_text(child, source).trim_start_matches('$');
                let range = locals::node_to_scip_range(child);
                ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
            } else {
                // Might be wrapped in another node — look for variable_name children
                for j in 0..child.named_child_count() {
                    if let Some(grandchild) = child.named_child(j) {
                        if grandchild.kind() == "variable_name" {
                            let name = node_text(grandchild, source).trim_start_matches('$');
                            let range = locals::node_to_scip_range(grandchild);
                            ctx.local_tracker.define_variable(name, range, &mut ctx.local_counter);
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Composer;
    use crate::parser::PhpParser;
    use crate::symbol::namer::SymbolNamer;
    use crate::types::TypeDatabase;

    fn setup_and_index(php_source: &str) -> FileResult {
        setup_and_index_with_db(php_source, TypeDatabase::new())
    }

    fn setup_and_index_with_db(php_source: &str, type_db: TypeDatabase) -> FileResult {
        let mut parser = PhpParser::new();
        let parsed = parser.parse(php_source, "test.php").unwrap();

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

    #[test]
    fn test_traversal_simple_class() {
        // Verify scope depth returns to 0 for a class with a method.
        // The debug_assert in index_file checks this; if scope tracking is
        // broken, this test will panic.
        let source = r#"<?php
namespace App\Models;

class User {
    public function getName(): string {
        return $this->name;
    }
}
"#;
        let result = setup_and_index(source);
        // Traversal completed without panicking; definitions are now emitted
        assert!(!result.occurrences.is_empty());
        assert!(!result.symbols.is_empty());
    }

    #[test]
    fn test_traversal_nested_closures() {
        // Function containing a closure and an arrow function — verify scope balancing.
        let source = r#"<?php
function doStuff() {
    $fn1 = function($x) {
        return $x + 1;
    };

    $fn2 = fn($y) => $y * 2;
}
"#;
        let result = setup_and_index(source);
        // No panics from scope depth assertion; definitions are now emitted
        assert!(!result.occurrences.is_empty());
    }

    #[test]
    fn test_traversal_error_nodes_skipped() {
        // PHP with a syntax error should not panic during traversal
        let source = r#"<?php
class Foo {
    public function bar( {
        return 42;
    }
}
"#;
        let result = setup_and_index(source);
        // Should complete without panicking; some definitions may be emitted
        // even with syntax errors
        let _ = result;
    }

    #[test]
    fn test_traversal_interface_trait_enum() {
        let source = r#"<?php
namespace App;

interface Cacheable {
    public function getCacheKey(): string;
}

trait Loggable {
    public function log(): void {}
}

enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
}
"#;
        let result = setup_and_index(source);
        // Traversal completes, scope depth returns to 0; definitions are emitted
        assert!(!result.occurrences.is_empty());
    }

    #[test]
    fn test_traversal_multiple_classes() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {}
}

class Baz extends Foo {
    public function qux(): void {
        $cb = function() {
            return fn() => 42;
        };
    }
}
"#;
        let result = setup_and_index(source);
        // Definitions are now emitted
        assert!(!result.occurrences.is_empty());
    }

    #[test]
    fn test_traversal_empty_file() {
        let result = setup_and_index("<?php\n");
        assert!(result.occurrences.is_empty());
        assert!(result.symbols.is_empty());
    }

    #[test]
    fn test_traversal_global_namespace() {
        let source = r#"<?php
class SimpleClass {
    public function method(): void {}
}

function standalone_func() {
    return 1;
}
"#;
        let result = setup_and_index(source);
        // Definitions are now emitted for class, method, and function
        assert!(!result.occurrences.is_empty());
    }

    #[test]
    fn test_name_resolver_initialized() {
        // Verify that the pre-pass correctly initializes the resolver
        let source = r#"<?php
namespace App\Services;

use App\Models\User;
use Psr\Log\LoggerInterface as Logger;

class UserService {
    public function find(): void {}
}
"#;
        let mut parser = PhpParser::new();
        let parsed = parser.parse(source, "test.php").unwrap();

        let type_db = TypeDatabase::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();
        let composer = Composer::load(dir.path()).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let file_path = dir.path().join("src/UserService.php");

        let mut ctx = IndexingContext::new(
            &file_path,
            &parsed.source,
            &type_db,
            &composer,
            &namer,
            dir.path(),
        );

        index_file(&parsed, &mut ctx);

        // After indexing, the resolver should have the namespace and imports set up
        assert_eq!(ctx.resolver.namespace(), "App\\Services");
        assert_eq!(
            ctx.resolver.resolve_class("User"),
            "App\\Models\\User"
        );
        assert_eq!(
            ctx.resolver.resolve_class("Logger"),
            "Psr\\Log\\LoggerInterface"
        );
    }

    #[test]
    fn test_closure_inherits_this() {
        // Non-static closure inside a method should inherit $this
        let source = r#"<?php
namespace App;

class Processor {
    private string $name = '';
    public function process(): void {
        $fn = function() {
            $this->name;
        };
    }
}
"#;
        let result = setup_and_index(source);
        // $this->name inside closure should produce a property reference
        let prop_refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("$name") && o.symbol_roles == crate::output::scip::symbol_roles::REFERENCE)
            .collect();
        assert!(
            !prop_refs.is_empty(),
            "Expected property reference for $this->name inside closure"
        );
    }

    #[test]
    fn test_static_closure_no_this() {
        // Static closure should NOT have $this available
        let source = r#"<?php
namespace App;

class Foo {
    private string $name = '';
    public function test(): void {
        $fn = static function() {
            // $this is not available here
        };
    }
}
"#;
        // Should not panic
        let result = setup_and_index(source);
        assert!(!result.occurrences.is_empty());
    }

    #[test]
    fn test_fluent_interface_chain() {
        // Test that $this->method() chains resolve correctly when
        // method return types are "self"
        let source = r#"<?php
namespace App;

class QueryBuilder {
    public function where(string $col): self { return $this; }
    public function limit(int $n): self { return $this; }
    public function execute(): array { return []; }
    public function test(): void {
        $this->where('a')->limit(10)->execute();
    }
}
"#;
        // We need a TypeDatabase with method return types for fluent chain
        let mut type_db = TypeDatabase::new();
        type_db.add_method("App\\QueryBuilder", "where", Some("self".to_string()), vec![]);
        type_db.add_method("App\\QueryBuilder", "limit", Some("self".to_string()), vec![]);
        type_db.add_method("App\\QueryBuilder", "execute", Some("array".to_string()), vec![]);
        crate::types::upper_chain::build_transitive_uppers(&mut type_db);

        let result = setup_and_index_with_db(source, type_db);
        let ref_role = crate::output::scip::symbol_roles::REFERENCE;
        // All three method calls should produce references
        let where_refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("where().") && o.symbol_roles == ref_role)
            .collect();
        let limit_refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("limit().") && o.symbol_roles == ref_role)
            .collect();
        let execute_refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("execute().") && o.symbol_roles == ref_role)
            .collect();
        assert!(
            !where_refs.is_empty(),
            "Expected reference for where() in fluent chain"
        );
        assert!(
            !limit_refs.is_empty(),
            "Expected reference for limit() in fluent chain"
        );
        assert!(
            !execute_refs.is_empty(),
            "Expected reference for execute() in fluent chain"
        );
    }

    #[test]
    fn test_this_method_call_in_method() {
        // $this->method() inside a method body should resolve to the current class
        let source = r#"<?php
namespace App;

class Builder {
    public function where(string $col): self { return $this; }
    public function build(): void {
        $this->where('id');
    }
}
"#;
        let result = setup_and_index(source);
        let refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("where().") && o.symbol_roles == crate::output::scip::symbol_roles::REFERENCE)
            .collect();
        assert!(
            !refs.is_empty(),
            "Expected method reference for $this->where() call"
        );
    }

    #[test]
    fn test_phpdoc_param_fallback() {
        // When a parameter has no native type hint, PHPDoc @param should be used
        let source = r#"<?php
namespace App;

use App\Services\UserService;

class Processor {
    /**
     * @param UserService $svc The service
     */
    public function handle($svc): void {
        $svc->process();
    }
}
"#;
        // We need UserService.process in the TypeDatabase for the reference to emit
        let mut type_db = TypeDatabase::new();
        type_db.add_method("App\\Services\\UserService", "process", Some("void".to_string()), vec![]);
        crate::types::upper_chain::build_transitive_uppers(&mut type_db);

        let result = setup_and_index_with_db(source, type_db);
        let refs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.contains("process().") && o.symbol_roles == crate::output::scip::symbol_roles::REFERENCE)
            .collect();
        assert!(
            !refs.is_empty(),
            "Expected method reference for $svc->process() via PHPDoc @param fallback"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Local variable tracking integration tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_local_var_simple_assignment() {
        let source = r#"<?php
function run(): void {
    $x = 'hello';
    $x = 'world';
}
"#;
        let result = setup_and_index(source);
        let locals: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local "))
            .collect();
        // Expect: param-less function, so $x first = definition, $x second = reference
        let defs: Vec<_> = locals.iter()
            .filter(|o| o.symbol_roles == crate::output::scip::symbol_roles::DEFINITION)
            .collect();
        let refs: Vec<_> = locals.iter()
            .filter(|o| o.symbol_roles == crate::output::scip::symbol_roles::REFERENCE)
            .collect();
        assert!(!defs.is_empty(), "Expected at least one local definition for $x");
        assert!(!refs.is_empty(), "Expected at least one local reference for $x re-assignment");
    }

    #[test]
    fn test_local_var_this_not_tracked() {
        let source = r#"<?php
class Foo {
    public function run(): void {
        $this->bar();
    }
}
"#;
        let result = setup_and_index(source);
        let this_locals: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local ") && o.range[0] == 3) // line 3: $this->bar()
            .collect();
        // $this should NOT be tracked as a local variable
        assert!(this_locals.is_empty(), "Expected $this NOT to be tracked as local variable");
    }

    #[test]
    fn test_local_var_foreach_key_value() {
        let source = r#"<?php
function run(): void {
    $items = [];
    foreach ($items as $key => $value) {
        echo $key;
    }
}
"#;
        let result = setup_and_index(source);
        let local_defs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local ") && o.symbol_roles == crate::output::scip::symbol_roles::DEFINITION)
            .collect();
        // Should have definitions for $items, $key, $value (at minimum)
        assert!(local_defs.len() >= 3,
            "Expected at least 3 local definitions ($items, $key, $value), got {}",
            local_defs.len());
    }

    #[test]
    fn test_local_var_catch_variable() {
        let source = r#"<?php
function run(): void {
    try {
        throw new \Exception();
    } catch (\Exception $e) {
        echo $e;
    }
}
"#;
        let result = setup_and_index(source);
        let catch_defs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local ") && o.symbol_roles == crate::output::scip::symbol_roles::DEFINITION)
            .collect();
        // Should have at least a definition for $e
        assert!(!catch_defs.is_empty(), "Expected local definition for catch variable $e");
    }

    #[test]
    fn test_local_var_list_destructuring() {
        let source = r#"<?php
function run(): void {
    [$first, $second] = [1, 2];
}
"#;
        let result = setup_and_index(source);
        let local_defs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local ") && o.symbol_roles == crate::output::scip::symbol_roles::DEFINITION)
            .collect();
        assert!(local_defs.len() >= 2,
            "Expected at least 2 local definitions ($first, $second), got {}",
            local_defs.len());
    }

    #[test]
    fn test_local_var_param_registered() {
        let source = r#"<?php
function greet(string $name): void {
    $greeting = 'Hello ' . $name;
}
"#;
        let result = setup_and_index(source);
        // $name param definition was emitted by emit_param_definition (Task 10)
        // $greeting should be a new local definition
        // $name in the function body should resolve as a reference (not a new definition)
        let local_defs: Vec<_> = result.occurrences.iter()
            .filter(|o| o.symbol.starts_with("local ") && o.symbol_roles == crate::output::scip::symbol_roles::DEFINITION)
            .collect();
        // At least: $name param def + $greeting def = 2 definitions
        assert!(local_defs.len() >= 2,
            "Expected at least 2 local definitions ($name param + $greeting), got {}",
            local_defs.len());
    }
}
