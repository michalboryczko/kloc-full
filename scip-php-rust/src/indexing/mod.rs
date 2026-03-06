//! SCIP occurrence and symbol emission (Pass 2 indexing).
//!
//! Contains the CST traversal driver (`index_file`), per-file indexing context,
//! and stub emitters for definitions and references.

pub mod context;
pub mod definitions;
pub mod references;

// Re-exports
pub use context::{FileResult, IndexingContext};

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

    let mut did_enter = true;

    loop {
        if did_enter {
            let node = cursor.node();

            // Enter: classify and potentially push scope
            let pushed = enter_node(node, source, ctx);
            scope_pushed_stack.push(pushed);
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

/// Process a node on entry during traversal.
///
/// Returns `true` if a scope frame was pushed (must be popped on exit).
fn enter_node(node: tree_sitter::Node, source: &[u8], ctx: &mut IndexingContext) -> bool {
    // Skip error nodes to avoid panics on malformed input
    if node.has_error() && node.kind() == "ERROR" {
        return false;
    }

    let php_node = classify_node(node, source);

    match &php_node {
        // --- Scope-defining nodes: push scope frames ---
        PhpNode::Namespace(ns) => {
            let ns_name = ns.name();
            ctx.scope.push_namespace(ns_name.to_string());
            return true;
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
            return true;
        }

        PhpNode::Method(method_node) => {
            let name = method_node.name().to_string();
            let is_static = method_node.is_static();
            ctx.scope.push_method(name, is_static);

            // Call stub definition emitter
            definitions::emit_method_definition(method_node, ctx);
            return true;
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

            // Call stub definition emitter
            definitions::emit_function_definition(func_node, ctx);
            return true;
        }

        PhpNode::Closure(_) => {
            ctx.scope.push_closure();
            return true;
        }

        PhpNode::ArrowFunction(_) => {
            ctx.scope.push_arrow_function();
            return true;
        }

        // --- Definition nodes: call stub emitters ---
        PhpNode::Param(param_node) => {
            definitions::emit_param_definition(param_node, ctx);
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

        // --- Expression/reference nodes: call stub handler ---
        PhpNode::MethodCall(_)
        | PhpNode::StaticCall(_)
        | PhpNode::FuncCall(_)
        | PhpNode::New(_)
        | PhpNode::PropertyFetch(_)
        | PhpNode::StaticPropertyFetch(_)
        | PhpNode::ClassConstFetch(_)
        | PhpNode::Variable(_)
        | PhpNode::Assign(_)
        | PhpNode::Foreach(_)
        | PhpNode::Name(_) => {
            references::handle_expression(&php_node, ctx);
        }

        // Everything else — check for trait use declarations, instanceof, catch
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

            // Handle `catch (Exception $e)` — emit reference for exception class
            if node.kind() == "catch_clause" {
                references::handle_catch_clause(node, ctx);
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Composer;
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
}
