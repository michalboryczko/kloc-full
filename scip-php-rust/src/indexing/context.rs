//! Per-file indexing context: mutable state and output accumulators.

use std::path::Path;

use crate::composer::Composer;
use crate::indexing::calls::{CallRecord, ValueRecord};
use crate::indexing::expression_tracker::ExpressionTracker;
use crate::indexing::locals::LocalVariableTracker;
use crate::names::resolver::NameResolver;
use crate::output::scip::{Occurrence, Relationship, SymbolInformation, symbol_roles};
use crate::symbol::namer::SymbolNamer;
use crate::symbol::scope::ScopeStack;
use crate::types::TypeDatabase;

/// All SCIP output produced for a single PHP file.
pub struct FileResult {
    pub relative_path: String,
    pub occurrences: Vec<Occurrence>,
    pub symbols: Vec<SymbolInformation>,
    pub calls: Vec<CallRecord>,
    pub values: Vec<ValueRecord>,
}

/// Per-file mutable state for indexing. Created fresh for each file.
pub struct IndexingContext<'a> {
    // Per-file mutable state
    pub resolver: NameResolver,
    pub scope: ScopeStack,
    /// Per-callable variable type map. Reset when entering a new callable scope.
    pub var_types: crate::types::resolver::VariableTypeMap,

    // Output accumulators
    occurrences: Vec<Occurrence>,
    symbols: Vec<SymbolInformation>,
    calls: Vec<CallRecord>,
    values: Vec<ValueRecord>,

    // Expression tracker for call/value records
    pub expression_tracker: ExpressionTracker,

    // Local variable tracker
    pub local_tracker: LocalVariableTracker,

    // Local variable counter
    pub local_counter: u32,

    // Shared read-only resources
    pub type_db: &'a TypeDatabase,
    pub composer: &'a Composer,
    pub namer: &'a SymbolNamer,

    // File metadata
    pub file_path: &'a Path,
    pub relative_path: String,
    pub source: &'a [u8],
}

impl<'a> IndexingContext<'a> {
    /// Create a fresh indexing context for a file.
    ///
    /// `project_root` is used to compute the relative path.
    pub fn new(
        file_path: &'a Path,
        source: &'a [u8],
        type_db: &'a TypeDatabase,
        composer: &'a Composer,
        namer: &'a SymbolNamer,
        project_root: &Path,
    ) -> Self {
        let relative_path = file_path
            .strip_prefix(project_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        IndexingContext {
            resolver: NameResolver::new(),
            scope: ScopeStack::new(),
            var_types: crate::types::resolver::VariableTypeMap::new(),
            occurrences: Vec::new(),
            symbols: Vec::new(),
            calls: Vec::new(),
            values: Vec::new(),
            expression_tracker: ExpressionTracker::new(),
            local_tracker: LocalVariableTracker::new(),
            local_counter: 0,
            type_db,
            composer,
            namer,
            file_path,
            relative_path,
            source,
        }
    }

    /// Add a definition occurrence with its symbol information.
    ///
    /// Creates both an `Occurrence` (with definition role) and a `SymbolInformation`
    /// entry for the symbol.
    pub fn add_definition(
        &mut self,
        symbol: String,
        range: Vec<u32>,
        roles: u32,
        documentation: Vec<String>,
        relationships: Vec<Relationship>,
    ) {
        self.occurrences.push(Occurrence {
            range,
            symbol: symbol.clone(),
            symbol_roles: roles | symbol_roles::DEFINITION,
            override_documentation: Vec::new(),
            diagnostics: Vec::new(),
            enclosing_range: Vec::new(),
        });

        self.symbols.push(SymbolInformation {
            symbol,
            documentation,
            relationships,
            kind: None,
        });
    }

    /// Add a reference occurrence (no symbol information entry).
    pub fn add_reference(&mut self, symbol: String, range: Vec<u32>) {
        self.occurrences.push(Occurrence {
            range,
            symbol,
            symbol_roles: symbol_roles::REFERENCE,
            override_documentation: Vec::new(),
            diagnostics: Vec::new(),
            enclosing_range: Vec::new(),
        });
    }

    /// Add a call record.
    pub fn add_call(&mut self, call: CallRecord) {
        self.calls.push(call);
    }

    /// Add a value record.
    pub fn add_value(&mut self, value: ValueRecord) {
        self.values.push(value);
    }

    /// Get the next local variable ID and increment the counter.
    pub fn next_local_id(&mut self) -> u32 {
        let id = self.local_counter;
        self.local_counter += 1;
        id
    }

    /// Consume the context and return the collected file result.
    pub fn into_result(mut self) -> FileResult {
        // Convert local variable occurrences into SCIP occurrences
        for local_occ in self.local_tracker.local_occurrences.drain(..) {
            let role = if local_occ.is_definition {
                symbol_roles::DEFINITION
            } else {
                symbol_roles::REFERENCE
            };
            self.occurrences.push(Occurrence {
                range: local_occ.range,
                symbol: local_occ.symbol,
                symbol_roles: role,
                override_documentation: Vec::new(),
                diagnostics: Vec::new(),
                enclosing_range: Vec::new(),
            });
        }

        FileResult {
            relative_path: self.relative_path,
            occurrences: self.occurrences,
            symbols: self.symbols,
            calls: self.calls,
            values: self.values,
        }
    }

    /// Get the FQN of the current class from the scope stack, if any.
    pub fn current_class(&self) -> Option<&str> {
        self.scope.current_class()
    }

    /// Get the name of the current method from the scope stack, if any.
    pub fn current_method(&self) -> Option<&str> {
        self.scope.current_method()
    }

    /// Get the SCIP-format caller symbol for the current callable context.
    ///
    /// Returns `None` if outside any callable scope.
    pub fn caller_symbol(&self) -> Option<String> {
        self.scope.current_callable_fqn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Composer;
    use crate::symbol::namer::SymbolNamer;
    use crate::types::TypeDatabase;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_context<'a>(
        source: &'a [u8],
        file_path: &'a Path,
        type_db: &'a TypeDatabase,
        composer: &'a Composer,
        namer: &'a SymbolNamer,
        project_root: &Path,
    ) -> IndexingContext<'a> {
        IndexingContext::new(file_path, source, type_db, composer, namer, project_root)
    }

    fn setup_minimal_project() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"name": "test/project", "version": "1.0.0"}"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_file_result_creation() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php class Foo {}";
        let file_path = project_root.join("src/Foo.php");

        let mut ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        // Add a definition
        ctx.add_definition(
            "scip-php composer test/project 1.0.0 Foo#".to_string(),
            vec![0, 6, 0, 9],
            0,
            vec!["class Foo".to_string()],
            Vec::new(),
        );

        // Add a reference
        ctx.add_reference(
            "scip-php composer test/project 1.0.0 Foo#".to_string(),
            vec![1, 0, 1, 3],
        );

        let result = ctx.into_result();
        assert_eq!(result.relative_path, "src/Foo.php");
        assert_eq!(result.occurrences.len(), 2);
        assert_eq!(result.symbols.len(), 1);

        // First occurrence should be a definition
        assert_ne!(
            result.occurrences[0].symbol_roles & symbol_roles::DEFINITION,
            0
        );
        // Second occurrence should be a reference
        assert_eq!(
            result.occurrences[1].symbol_roles,
            symbol_roles::REFERENCE
        );
    }

    #[test]
    fn test_relative_path_calculation() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php";
        let file_path = project_root.join("src/Models/User.php");

        let ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        assert_eq!(ctx.relative_path, "src/Models/User.php");
    }

    #[test]
    fn test_relative_path_outside_project() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php";
        let file_path = PathBuf::from("/tmp/outside/file.php");

        let ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        // When file is outside project root, full path is used
        assert_eq!(ctx.relative_path, "/tmp/outside/file.php");
    }

    #[test]
    fn test_local_counter() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php";
        let file_path = project_root.join("test.php");

        let mut ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        assert_eq!(ctx.next_local_id(), 0);
        assert_eq!(ctx.next_local_id(), 1);
        assert_eq!(ctx.next_local_id(), 2);
        assert_eq!(ctx.local_counter, 3);
    }

    #[test]
    fn test_scope_delegation() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php";
        let file_path = project_root.join("test.php");

        let mut ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        // Initially no class or method
        assert!(ctx.current_class().is_none());
        assert!(ctx.current_method().is_none());
        assert!(ctx.caller_symbol().is_none());

        // Push some scope frames and check delegation
        ctx.scope.push_class(
            "App\\Foo".to_string(),
            crate::symbol::scope::ClassKind::Class,
        );
        assert_eq!(ctx.current_class(), Some("App\\Foo"));

        ctx.scope.push_method("bar".to_string(), false);
        assert_eq!(ctx.current_method(), Some("bar"));
        assert_eq!(
            ctx.caller_symbol(),
            Some("App\\Foo::bar".to_string())
        );
    }

    #[test]
    fn test_add_call_and_value() {
        let dir = setup_minimal_project();
        let project_root = dir.path();
        let type_db = TypeDatabase::new();
        let composer = Composer::load(project_root).unwrap();
        let namer = SymbolNamer::new("test/project", "1.0.0");
        let source = b"<?php";
        let file_path = project_root.join("test.php");

        let mut ctx = setup_test_context(
            source,
            &file_path,
            &type_db,
            &composer,
            &namer,
            project_root,
        );

        ctx.add_call(CallRecord {
            caller: "App\\Foo::bar".to_string(),
            callee: "App\\Baz::qux".to_string(),
            kind: crate::indexing::calls::CallKind::MethodCall,
            file: "test.php".to_string(),
            line: 5,
            arguments: vec![],
        });

        ctx.add_value(ValueRecord {
            source: "App\\Foo::bar".to_string(),
            target: "App\\Foo#$name.".to_string(),
            kind: crate::indexing::calls::ValueKind::PropertyRead,
            file: "test.php".to_string(),
            line: 3,
        });

        let result = ctx.into_result();
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].callee, "App\\Baz::qux");
        assert_eq!(result.values.len(), 1);
        assert_eq!(result.values[0].target, "App\\Foo#$name.");
    }
}
