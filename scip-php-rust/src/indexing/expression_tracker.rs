//! Expression tracker: produces CallRecord and ValueRecord entries
//! from PHP call expressions during indexing.

use crate::indexing::calls::{ArgumentValue, CallKind, CallRecord, ValueKind, ValueRecord};
use crate::names::resolver::NameResolver;
use crate::parser::ast::{
    ClassConstFetchNode, FuncCallNode, MethodCallNode, NewNode, PropertyFetchNode,
    StaticCallNode, StaticPropertyFetchNode,
};
use crate::parser::cst::node_text;
use crate::symbol::scope::ScopeStack;
use crate::types::TypeDatabase;
use crate::types::resolver::VariableTypeMap;

/// Accumulates call and value records during file indexing.
#[derive(Debug, Default)]
pub struct ExpressionTracker {
    pub call_records: Vec<CallRecord>,
    pub value_records: Vec<ValueRecord>,
}

impl ExpressionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Track a method call expression: `$obj->method()` or `$obj?->method()`.
    ///
    /// Resolves the object type to determine the callee class.
    /// Unknown receivers produce an `"unknown#methodName()."` sentinel.
    pub fn track_method_call(
        &mut self,
        node: &MethodCallNode,
        source: &[u8],
        scope: &ScopeStack,
        var_types: &VariableTypeMap,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        // Skip dynamic method calls: $obj->$method()
        let method_name_node = match node.method_name_node() {
            Some(n) => n,
            None => return,
        };
        if method_name_node.kind() == "variable_name" {
            return; // dynamic: $obj->$method()
        }

        let method_name = match node.method_name() {
            Some(n) => n,
            None => return,
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return, // outside callable scope
        };

        let object_node = match node.object() {
            Some(n) => n,
            None => return,
        };

        let kind = if node.is_nullsafe() {
            CallKind::NullsafeMethodCall
        } else {
            CallKind::MethodCall
        };

        // Resolve the receiver type
        let receiver_type = crate::types::resolver::resolve_expr_type(
            object_node,
            source,
            var_types,
            scope,
            type_db,
            resolver,
        );

        let callee = if let Some(class_fqn) = receiver_type {
            let class_fqn = crate::types::resolver::strip_nullable(&class_fqn).to_string();
            namer.symbol_for_method(&class_fqn, method_name, pkg, ver)
        } else {
            format!("unknown#{}().", method_name)
        };

        let caller = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);

        // Line is 0-indexed in tree-sitter, we want 1-indexed
        let line = node.node.start_position().row as u32 + 1;

        let arguments = match node.arguments() {
            Some(args_node) => Self::extract_arguments(
                args_node, source, var_types, scope, type_db, resolver,
            ),
            None => Vec::new(),
        };

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind,
            file: relative_path.to_string(),
            line,
            arguments,
        });
    }

    /// Track a static call expression: `Foo::method()`.
    ///
    /// Handles self/static/parent resolution.
    pub fn track_static_call(
        &mut self,
        node: &StaticCallNode,
        source: &[u8],
        scope: &ScopeStack,
        var_types: &VariableTypeMap,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let scope_text = match node.scope_text() {
            Some(s) => s,
            None => return,
        };
        let method_name = match node.method_name() {
            Some(n) => n,
            None => return, // dynamic: Foo::$method()
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        // Resolve the class FQN (handles self/static/parent)
        let class_fqn = match resolve_scope_class(scope_text, scope, type_db, resolver) {
            Some(fqn) => fqn,
            None => return,
        };

        let callee = namer.symbol_for_method(&class_fqn, method_name, pkg, ver);
        let caller = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);
        let line = node.node.start_position().row as u32 + 1;

        let arguments = match node.arguments() {
            Some(args_node) => Self::extract_arguments(
                args_node, source, var_types, scope, type_db, resolver,
            ),
            None => Vec::new(),
        };

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind: CallKind::StaticCall,
            file: relative_path.to_string(),
            line,
            arguments,
        });
    }

    /// Track a function call expression: `func_name()`.
    ///
    /// Skips variable function calls like `$func()`.
    pub fn track_func_call(
        &mut self,
        node: &FuncCallNode,
        source: &[u8],
        scope: &ScopeStack,
        var_types: &VariableTypeMap,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let func_name = match node.function_name() {
            Some(name) => name,
            None => return, // dynamic: $func()
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        let resolution = resolver.resolve_function(func_name);
        let fqn = resolution.primary().to_string();
        let callee = namer.symbol_for_function(&fqn, pkg, ver);
        let caller = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);
        let line = node.node.start_position().row as u32 + 1;

        let arguments = match node.arguments() {
            Some(args_node) => Self::extract_arguments(
                args_node, source, var_types, scope, type_db, resolver,
            ),
            None => Vec::new(),
        };

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind: CallKind::FuncCall,
            file: relative_path.to_string(),
            line,
            arguments,
        });
    }

    /// Track a new expression: `new Foo()`.
    ///
    /// The callee is the constructor (`__construct`) of the resolved class.
    pub fn track_new_call(
        &mut self,
        node: &NewNode,
        _source: &[u8],
        scope: &ScopeStack,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let class_name = match node.class_name() {
            Some(name) => name,
            None => return, // dynamic: new $className()
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        // Resolve class FQN (handle self/static/parent)
        let class_fqn = if matches!(class_name, "self" | "static") {
            match scope.current_class() {
                Some(c) => c.to_string(),
                None => return,
            }
        } else if class_name == "parent" {
            // Skip parent — can't easily resolve without TypeDatabase here
            return;
        } else {
            resolver.resolve_class(class_name)
        };

        if class_fqn.is_empty() {
            return;
        }

        let callee = namer.symbol_for_constructor(&class_fqn, pkg, ver);
        let caller = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);
        let line = node.node.start_position().row as u32 + 1;

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind: CallKind::New,
            file: relative_path.to_string(),
            line,
            arguments: Vec::new(),
        });
    }

    /// Track a property access: `$obj->prop` or `$obj?->prop`.
    ///
    /// Emits a ValueRecord with PropertyRead or PropertyWrite kind.
    /// Skips if the object type cannot be resolved.
    pub fn track_property_access(
        &mut self,
        node: &PropertyFetchNode,
        source: &[u8],
        scope: &ScopeStack,
        var_types: &VariableTypeMap,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let prop_name = match node.property_name() {
            Some(n) => n,
            None => return,
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        let object_node = match node.object() {
            Some(n) => n,
            None => return,
        };

        let receiver_type = crate::types::resolver::resolve_expr_type(
            object_node, source, var_types, scope, type_db, resolver,
        );

        let class_fqn = match receiver_type {
            Some(ref t) => crate::types::resolver::strip_nullable(t).to_string(),
            None => return,
        };

        let target = namer.symbol_for_property(&class_fqn, prop_name, pkg, ver);
        let source_sym = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);

        let kind = if is_write_context(node.node) {
            ValueKind::PropertyWrite
        } else {
            ValueKind::PropertyRead
        };

        let line = node.node.start_position().row as u32 + 1;

        self.value_records.push(ValueRecord {
            source: source_sym,
            target,
            kind,
            file: relative_path.to_string(),
            line,
        });
    }

    /// Track a static property access: `Foo::$prop`.
    ///
    /// Handles self/static/parent resolution.
    pub fn track_static_property_access(
        &mut self,
        node: &StaticPropertyFetchNode,
        _source: &[u8],
        scope: &ScopeStack,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let scope_text = match node.scope_text() {
            Some(s) => s,
            None => return,
        };
        let prop_name = match node.property_name() {
            Some(n) => n,
            None => return,
        };

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        let class_fqn = match resolve_scope_class(scope_text, scope, type_db, resolver) {
            Some(fqn) => fqn,
            None => return,
        };

        // Strip leading $ from property name
        let prop_name_clean = prop_name.trim_start_matches('$');
        let target = namer.symbol_for_property(&class_fqn, prop_name_clean, pkg, ver);
        let source_sym = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);

        let kind = if is_write_context(node.node) {
            ValueKind::StaticPropertyWrite
        } else {
            ValueKind::StaticPropertyRead
        };

        let line = node.node.start_position().row as u32 + 1;

        self.value_records.push(ValueRecord {
            source: source_sym,
            target,
            kind,
            file: relative_path.to_string(),
            line,
        });
    }

    /// Track a class constant access: `Foo::CONST`.
    ///
    /// Skips `::class` pseudo-constant.
    pub fn track_class_const_access(
        &mut self,
        node: &ClassConstFetchNode,
        _source: &[u8],
        scope: &ScopeStack,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
        relative_path: &str,
        pkg: &str,
        ver: &str,
        namer: &crate::symbol::namer::SymbolNamer,
    ) {
        let scope_text = match node.scope_text() {
            Some(s) => s,
            None => return,
        };
        let const_name = match node.const_name() {
            Some(n) => n,
            None => return,
        };

        // Skip ::class pseudo-constant
        if const_name == "class" {
            return;
        }

        let caller_fqn = match scope.current_callable_fqn() {
            Some(fqn) => fqn,
            None => return,
        };

        let class_fqn = match resolve_scope_class(scope_text, scope, type_db, resolver) {
            Some(fqn) => fqn,
            None => return,
        };

        let target = namer.symbol_for_class_const(&class_fqn, const_name, pkg, ver);
        let source_sym = build_caller_symbol(&caller_fqn, scope, namer, pkg, ver);

        let line = node.node.start_position().row as u32 + 1;

        self.value_records.push(ValueRecord {
            source: source_sym,
            target,
            kind: ValueKind::ClassConstRead,
            file: relative_path.to_string(),
            line,
        });
    }

    /// Extract ArgumentValues from a call's arguments node.
    ///
    /// Iterates the named children of an `arguments` node. Named arguments
    /// extract only the value field; spread arguments are skipped.
    pub fn extract_arguments(
        arguments_node: tree_sitter::Node,
        source: &[u8],
        var_types: &VariableTypeMap,
        scope: &ScopeStack,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
    ) -> Vec<ArgumentValue> {
        let mut result = Vec::new();
        for i in 0..arguments_node.named_child_count() {
            let arg = match arguments_node.named_child(i) {
                Some(a) => a,
                None => continue,
            };

            // Named argument: extract only the value expression
            let value_node = if arg.kind() == "named_argument" {
                match arg.child_by_field_name("value") {
                    Some(v) => v,
                    None => continue,
                }
            } else {
                arg
            };

            // Skip spread arguments
            if value_node.kind() == "variadic_unpacking" || value_node.kind() == "spread_element" {
                continue;
            }

            result.push(Self::track_value(value_node, source, var_types, scope, type_db, resolver));
        }
        result
    }

    /// Convert a single expression node to an ArgumentValue.
    /// Matches PHP ExpressionTracker::trackValue() 14-case dispatch.
    fn track_value(
        node: tree_sitter::Node,
        source: &[u8],
        var_types: &VariableTypeMap,
        scope: &ScopeStack,
        type_db: &TypeDatabase,
        resolver: &NameResolver,
    ) -> ArgumentValue {
        match node.kind() {
            // Case 1: String literal
            "string" => {
                let text = node_text(node, source);
                let value = strip_string_quotes(text);
                ArgumentValue::StringLiteral {
                    value: value.to_string(),
                }
            }
            // Encapsed string (double-quoted with interpolation) — treat as Unknown
            "encapsed_string" => ArgumentValue::Unknown,

            // Case 2: Integer literal
            "integer" => {
                let text = node_text(node, source);
                let value = parse_php_int(text).unwrap_or(0);
                ArgumentValue::IntLiteral { value }
            }

            // Case 3: Float literal
            "float" => {
                let text = node_text(node, source);
                let value = text.replace('_', "").parse::<f64>().unwrap_or(0.0);
                ArgumentValue::FloatLiteral { value }
            }

            // Case 4: Boolean / Null constants
            "boolean" => {
                let text = node_text(node, source).to_ascii_lowercase();
                ArgumentValue::BoolLiteral {
                    value: text == "true",
                }
            }
            "null" => ArgumentValue::NullLiteral,

            // Case 5: Variable
            "variable_name" => {
                let name = node_text(node, source).to_string();
                ArgumentValue::Variable { name }
            }

            // Case 6: Class constant: Foo::BAR
            "class_constant_access_expression" => {
                let scope_node = node.named_child(0);
                let name_node = node.named_child(1);
                let scope_text = scope_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let const_name = name_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let class_fqn = resolver.resolve_class(scope_text);
                ArgumentValue::ClassConst {
                    class: class_fqn,
                    name: const_name.to_string(),
                }
            }

            // Case 7: Static property: Foo::$prop
            "scoped_property_access_expression" => {
                let scope_node = node.child_by_field_name("scope");
                let name_node = node.child_by_field_name("name");
                let scope_text = scope_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let prop_name = name_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let class_fqn = resolver.resolve_class(scope_text);
                ArgumentValue::StaticPropertyFetch {
                    class: class_fqn,
                    name: prop_name.to_string(),
                }
            }

            // Case 8: Property fetch: $obj->prop, $obj?->prop
            "member_access_expression" | "nullsafe_member_access_expression" => {
                let prop_name = node
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let object_type = node.child_by_field_name("object").and_then(|obj| {
                    crate::types::resolver::resolve_expr_type(
                        obj, source, var_types, scope, type_db, resolver,
                    )
                });
                ArgumentValue::PropertyFetch {
                    object_type,
                    name: prop_name.to_string(),
                }
            }

            // Case 9: Method call: $obj->method(), $obj?->method()
            "member_call_expression" | "nullsafe_member_call_expression" => {
                let method = node
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let object_type = node.child_by_field_name("object").and_then(|obj| {
                    crate::types::resolver::resolve_expr_type(
                        obj, source, var_types, scope, type_db, resolver,
                    )
                });
                ArgumentValue::MethodCall {
                    object_type,
                    method: method.to_string(),
                }
            }

            // Case 10: Static call: Foo::method()
            "scoped_call_expression" => {
                let scope_node = node.child_by_field_name("scope");
                let name_node = node.child_by_field_name("name");
                let scope_text = scope_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let method = name_node
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let class_fqn = resolver.resolve_class(scope_text);
                ArgumentValue::StaticCall {
                    class: class_fqn,
                    method: method.to_string(),
                }
            }

            // Case 11: Function call: func()
            "function_call_expression" => {
                let func_name = node
                    .child_by_field_name("function")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");
                let resolution = resolver.resolve_function(func_name);
                ArgumentValue::FuncCall {
                    function: resolution.primary().to_string(),
                }
            }

            // Case 12: new expression
            "object_creation_expression" => {
                // Look for a name-like child
                let mut class_fqn = String::new();
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                            let class_name = node_text(child, source);
                            class_fqn = resolver.resolve_class(class_name);
                            break;
                        }
                    }
                }
                ArgumentValue::New { class: class_fqn }
            }

            // Case 13: Array literal — recurse into elements
            "array_creation_expression" => {
                let mut elements = Vec::new();
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i) {
                        if child.kind() == "array_element_initializer" {
                            // Array element has value as last named child
                            // or child_by_field_name("value")
                            if let Some(val) = child.child_by_field_name("value") {
                                elements.push(Self::track_value(
                                    val, source, var_types, scope, type_db, resolver,
                                ));
                            } else if child.named_child_count() > 0 {
                                // Fallback: last named child is the value
                                let last_idx = child.named_child_count() - 1;
                                if let Some(val) = child.named_child(last_idx) {
                                    elements.push(Self::track_value(
                                        val, source, var_types, scope, type_db, resolver,
                                    ));
                                }
                            }
                        }
                    }
                }
                ArgumentValue::ArrayLiteral { elements }
            }

            // Case 14: Everything else
            _ => ArgumentValue::Unknown,
        }
    }

    /// Drain all collected records, returning (calls, values).
    pub fn drain(&mut self) -> (Vec<CallRecord>, Vec<ValueRecord>) {
        (
            std::mem::take(&mut self.call_records),
            std::mem::take(&mut self.value_records),
        )
    }
}

/// Check if a node is in write context (LHS of assignment_expression).
fn is_write_context(node: tree_sitter::Node<'_>) -> bool {
    if let Some(parent) = node.parent() {
        if parent.kind() == "assignment_expression" {
            if let Some(left) = parent.child_by_field_name("left") {
                return left.id() == node.id();
            }
        }
    }
    false
}

/// Strip surrounding quotes from a PHP string literal.
fn strip_string_quotes(text: &str) -> &str {
    if text.len() >= 2 {
        let first = text.as_bytes()[0];
        let last = text.as_bytes()[text.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return &text[1..text.len() - 1];
        }
    }
    text
}

/// Parse a PHP integer literal, handling hex/octal/binary/decimal and _ separators.
fn parse_php_int(text: &str) -> Option<i64> {
    let text = text.replace('_', "");
    if text.starts_with("0x") || text.starts_with("0X") {
        i64::from_str_radix(&text[2..], 16).ok()
    } else if text.starts_with("0b") || text.starts_with("0B") {
        i64::from_str_radix(&text[2..], 2).ok()
    } else if text.starts_with('0') && text.len() > 1 && !text.contains('.') {
        i64::from_str_radix(&text[1..], 8).ok()
    } else {
        text.parse().ok()
    }
}

/// Resolve a scope keyword (self/static/parent) or class name to a FQN.
fn resolve_scope_class(
    scope_text: &str,
    scope: &ScopeStack,
    type_db: &TypeDatabase,
    resolver: &NameResolver,
) -> Option<String> {
    let lower = scope_text.to_ascii_lowercase();
    match lower.as_str() {
        "self" | "static" => scope.current_class().map(|s| s.to_string()),
        "parent" => {
            let class_fqn = scope.current_class()?;
            let uppers = type_db.get_direct_uppers(class_fqn);
            uppers.first().map(|s| s.to_string())
        }
        _ => {
            let fqn = resolver.resolve_class(scope_text);
            if fqn.is_empty() {
                None
            } else {
                Some(fqn)
            }
        }
    }
}

/// Build a SCIP caller symbol from the caller FQN.
///
/// If the caller is a method (contains "::"), construct a method symbol.
/// If it's a function FQN, construct a function symbol.
/// Otherwise, return the raw FQN as-is (e.g., for closures).
fn build_caller_symbol(
    caller_fqn: &str,
    _scope: &ScopeStack,
    namer: &crate::symbol::namer::SymbolNamer,
    pkg: &str,
    ver: &str,
) -> String {
    if let Some(pos) = caller_fqn.find("::") {
        let class_fqn = &caller_fqn[..pos];
        let method_name = &caller_fqn[pos + 2..];
        namer.symbol_for_method(class_fqn, method_name, pkg, ver)
    } else if caller_fqn.starts_with("closure#") {
        // Closure — return raw identifier
        caller_fqn.to_string()
    } else {
        // Standalone function
        namer.symbol_for_function(caller_fqn, pkg, ver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Composer;
    use crate::indexing::{IndexingContext, index_file};
    use crate::parser::PhpParser;
    use crate::symbol::namer::SymbolNamer;
    use crate::types::TypeDatabase;

    fn setup_and_index(php_source: &str) -> (crate::indexing::context::FileResult, Vec<CallRecord>) {
        setup_and_index_with_db(php_source, TypeDatabase::new())
    }

    fn setup_and_index_with_db(
        php_source: &str,
        type_db: TypeDatabase,
    ) -> (crate::indexing::context::FileResult, Vec<CallRecord>) {
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
        let calls = ctx.expression_tracker.call_records.clone();
        let result = ctx.into_result();
        (result, calls)
    }

    #[test]
    fn test_this_method_call_tracking() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {}
    public function baz(): void {
        $this->bar();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let method_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::MethodCall)
            .collect();
        assert!(
            !method_calls.is_empty(),
            "Expected at least one method call record for $this->bar()"
        );
        let call = &method_calls[0];
        assert!(call.caller.contains("baz()."), "caller should be baz method");
        assert!(call.callee.contains("bar()."), "callee should be bar method");
        assert_eq!(call.line, 7);
    }

    #[test]
    fn test_static_call_tracking() {
        let source = r#"<?php
namespace App;

use App\Services\UserService;

class Controller {
    public function handle(): void {
        UserService::findAll();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let static_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::StaticCall)
            .collect();
        assert!(
            !static_calls.is_empty(),
            "Expected at least one static call record"
        );
        let call = &static_calls[0];
        assert!(
            call.callee.contains("findAll()."),
            "callee should contain findAll"
        );
        assert_eq!(call.line, 8);
    }

    #[test]
    fn test_self_static_call_tracking() {
        let source = r#"<?php
namespace App;

class Foo {
    public static function bar(): void {}
    public static function baz(): void {
        self::bar();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let static_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::StaticCall)
            .collect();
        assert!(
            !static_calls.is_empty(),
            "Expected static call for self::bar()"
        );
        let call = &static_calls[0];
        assert!(call.callee.contains("Foo"), "callee should resolve self to Foo");
        assert!(call.callee.contains("bar()."), "callee should be bar method");
    }

    #[test]
    fn test_func_call_tracking() {
        let source = r#"<?php
namespace App;

function helper(): void {}

class Foo {
    public function bar(): void {
        strlen('hello');
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let func_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::FuncCall)
            .collect();
        assert!(
            !func_calls.is_empty(),
            "Expected at least one function call record"
        );
        let strlen_calls: Vec<_> = func_calls
            .iter()
            .filter(|c| c.callee.contains("strlen"))
            .collect();
        assert!(
            !strlen_calls.is_empty(),
            "Expected function call record for strlen()"
        );
    }

    #[test]
    fn test_new_call_tracking() {
        let source = r#"<?php
namespace App;

use App\Models\User;

class Factory {
    public function create(): void {
        $u = new User();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let new_calls: Vec<_> = calls.iter().filter(|c| c.kind == CallKind::New).collect();
        assert!(
            !new_calls.is_empty(),
            "Expected at least one new call record for new User()"
        );
        let call = &new_calls[0];
        assert!(
            call.callee.contains("__construct()."),
            "callee should be __construct"
        );
        assert!(call.callee.contains("User"), "callee should contain User");
        assert_eq!(call.line, 8);
    }

    #[test]
    fn test_unknown_receiver_sentinel() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar($obj): void {
        $obj->someMethod();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let method_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::MethodCall)
            .collect();
        assert!(
            !method_calls.is_empty(),
            "Expected method call record even for unknown receiver"
        );
        let call = &method_calls[0];
        assert!(
            call.callee.starts_with("unknown#"),
            "unknown receiver should produce unknown# sentinel, got: {}",
            call.callee
        );
    }

    #[test]
    fn test_new_self_tracking() {
        let source = r#"<?php
namespace App;

class Foo {
    public static function create(): void {
        return new self();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        let new_calls: Vec<_> = calls.iter().filter(|c| c.kind == CallKind::New).collect();
        assert!(
            !new_calls.is_empty(),
            "Expected new call record for new self()"
        );
        let call = &new_calls[0];
        assert!(
            call.callee.contains("Foo"),
            "new self() should resolve to Foo"
        );
    }

    #[test]
    fn test_nullsafe_method_call_tracking() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {
        $this?->baz();
    }
    public function baz(): void {}
}
"#;
        let (_, calls) = setup_and_index(source);
        let nullsafe_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::NullsafeMethodCall)
            .collect();
        assert!(
            !nullsafe_calls.is_empty(),
            "Expected nullsafe method call record"
        );
    }

    #[test]
    fn test_call_file_path() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(): void {
        strlen('test');
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        assert!(!calls.is_empty());
        assert_eq!(calls[0].file, "test.php");
    }

    #[test]
    fn test_dynamic_calls_skipped() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar($func, $method): void {
        $func();
        $this->$method();
    }
}
"#;
        let (_, calls) = setup_and_index(source);
        // Dynamic calls should be skipped — no call records
        assert!(
            calls.is_empty(),
            "Expected no call records for dynamic calls, got: {:?}",
            calls
        );
    }

    #[test]
    fn test_build_caller_symbol_method() {
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let scope = ScopeStack::new();
        let sym = build_caller_symbol("App\\Foo::bar", &scope, &namer, "test/project", "1.0.0");
        assert_eq!(
            sym,
            "scip-php composer test/project 1.0.0 App/Foo#bar()."
        );
    }

    #[test]
    fn test_build_caller_symbol_function() {
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let scope = ScopeStack::new();
        let sym = build_caller_symbol(
            "App\\Helpers\\format_date",
            &scope,
            &namer,
            "test/project",
            "1.0.0",
        );
        assert_eq!(
            sym,
            "scip-php composer test/project 1.0.0 App/Helpers/format_date()."
        );
    }

    #[test]
    fn test_build_caller_symbol_closure() {
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let scope = ScopeStack::new();
        let sym = build_caller_symbol("closure#0", &scope, &namer, "test/project", "1.0.0");
        assert_eq!(sym, "closure#0");
    }

    #[test]
    fn test_chained_calls_produce_multiple_records() {
        let source = r#"<?php
namespace App;

class Builder {
    public function where(string $col): self { return $this; }
    public function limit(int $n): self { return $this; }
    public function test(): void {
        $this->where('a')->limit(10);
    }
}
"#;
        let mut type_db = TypeDatabase::new();
        type_db.add_method(
            "App\\Builder",
            "where",
            Some("self".to_string()),
            vec![],
        );
        type_db.add_method(
            "App\\Builder",
            "limit",
            Some("self".to_string()),
            vec![],
        );
        crate::types::upper_chain::build_transitive_uppers(&mut type_db);

        let (_, calls) = setup_and_index_with_db(source, type_db);
        let method_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.kind == CallKind::MethodCall)
            .collect();
        assert!(
            method_calls.len() >= 2,
            "Expected at least 2 method call records for chained calls, got {}",
            method_calls.len()
        );
    }
}
