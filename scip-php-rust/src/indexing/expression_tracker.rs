//! Expression tracker: produces CallRecord and ValueRecord entries
//! from PHP call expressions during indexing.

use crate::indexing::calls::{CallKind, CallRecord, ValueRecord};
use crate::names::resolver::NameResolver;
use crate::parser::ast::{FuncCallNode, MethodCallNode, NewNode, StaticCallNode};
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
        let method_name = match node.method_name() {
            Some(n) => n,
            None => return, // dynamic: $obj->$method()
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

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind,
            file: relative_path.to_string(),
            line,
            arguments: Vec::new(), // Stub: Dev-2 implements argument extraction
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

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind: CallKind::StaticCall,
            file: relative_path.to_string(),
            line,
            arguments: Vec::new(),
        });
    }

    /// Track a function call expression: `func_name()`.
    ///
    /// Skips variable function calls like `$func()`.
    pub fn track_func_call(
        &mut self,
        node: &FuncCallNode,
        _source: &[u8],
        scope: &ScopeStack,
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

        self.call_records.push(CallRecord {
            caller,
            callee,
            kind: CallKind::FuncCall,
            file: relative_path.to_string(),
            line,
            arguments: Vec::new(),
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

    /// Drain all collected records, returning (calls, values).
    pub fn drain(&mut self) -> (Vec<CallRecord>, Vec<ValueRecord>) {
        (
            std::mem::take(&mut self.call_records),
            std::mem::take(&mut self.value_records),
        )
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
