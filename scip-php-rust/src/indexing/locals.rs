//! Local variable tracking: scope management and variable definition detection.
//!
//! PHP variables are function-scoped (like JavaScript `var`). Each function,
//! method, or closure creates a new variable scope. Arrow functions inherit
//! their parent scope (transparent). This module tracks which variables are
//! defined in each scope and emits SCIP occurrences for them.

use std::collections::HashMap;

use crate::symbol::namer::SymbolNamer;

/// Tracks local variable definitions across nested scopes.
pub struct LocalVariableTracker {
    scope_stack: Vec<VariableScope>,
    pub local_occurrences: Vec<LocalOccurrence>,
}

struct VariableScope {
    /// Maps variable name (without $) to local symbol ID.
    definitions: HashMap<String, u32>,
    kind: ScopeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeKind {
    Function,
    Closure,
    ArrowFunction,
}

/// A SCIP occurrence for a local variable.
pub struct LocalOccurrence {
    /// SCIP symbol string, e.g. "local 5"
    pub symbol: String,
    /// SCIP range [line, col, end_line, end_col] (0-indexed)
    pub range: Vec<u32>,
    pub is_definition: bool,
}

impl LocalVariableTracker {
    pub fn new() -> Self {
        LocalVariableTracker {
            scope_stack: Vec::new(),
            local_occurrences: Vec::new(),
        }
    }

    /// Push a new variable scope (function, method, or closure).
    pub fn enter_scope(&mut self, kind: ScopeKind) {
        self.scope_stack.push(VariableScope {
            definitions: HashMap::new(),
            kind,
        });
    }

    /// Pop the current variable scope.
    pub fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Arrow functions share the parent scope -- no-op.
    pub fn enter_arrow_function(&mut self) {
        // Arrow functions inherit all variables from enclosing scope.
    }

    /// Arrow functions share the parent scope -- no-op.
    pub fn exit_arrow_function(&mut self) {
        // No pop needed.
    }

    /// Define a variable in the current scope.
    ///
    /// If first time in this scope: allocates a new local ID, stores in definitions,
    /// emits a definition occurrence.
    /// If already defined: emits a reference occurrence (re-assignment).
    /// Skips `$this`.
    pub fn define_variable(
        &mut self,
        name: &str,
        range: Vec<u32>,
        local_counter: &mut u32,
    ) {
        if name == "this" {
            return;
        }

        let scope = match self.scope_stack.last_mut() {
            Some(s) => s,
            None => return,
        };

        if let Some(&existing_id) = scope.definitions.get(name) {
            // Re-assignment: emit reference
            self.local_occurrences.push(LocalOccurrence {
                symbol: SymbolNamer::symbol_for_local_var(existing_id),
                range,
                is_definition: false,
            });
        } else {
            // First definition: allocate local ID
            let id = *local_counter;
            *local_counter += 1;
            scope.definitions.insert(name.to_string(), id);
            self.local_occurrences.push(LocalOccurrence {
                symbol: SymbolNamer::symbol_for_local_var(id),
                range,
                is_definition: true,
            });
        }
    }

    /// Register a parameter as known in the current scope without emitting an occurrence.
    ///
    /// Task 10 already emitted the param definition occurrence. We just need to
    /// record it so that subsequent references to `$paramName` resolve correctly.
    pub fn register_param(&mut self, name: &str, local_id: u32) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.definitions.insert(name.to_string(), local_id);
        }
    }

    /// Look up a variable in the current scope (for reference resolution by Dev-2).
    pub fn lookup_variable(&self, name: &str) -> Option<u32> {
        // Walk scopes from innermost to outermost.
        // For closures, stop at the closure boundary (unless it's an arrow function scope).
        for scope in self.scope_stack.iter().rev() {
            if let Some(&id) = scope.definitions.get(name) {
                return Some(id);
            }
            // Function and Closure scopes are opaque -- stop looking.
            // Arrow functions are transparent (but we don't push them).
            match scope.kind {
                ScopeKind::Function | ScopeKind::Closure => break,
                ScopeKind::ArrowFunction => continue,
            }
        }
        None
    }

    /// Check if any scope is active.
    pub fn has_scope(&self) -> bool {
        !self.scope_stack.is_empty()
    }
}

impl Default for LocalVariableTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a tree-sitter node to a SCIP range [line, col, end_line, end_col].
pub fn node_to_scip_range(node: tree_sitter::Node) -> Vec<u32> {
    let start = node.start_position();
    let end = node.end_position();
    vec![
        start.row as u32,
        start.column as u32,
        end.row as u32,
        end.column as u32,
    ]
}

/// Recursively find variable_name nodes in a destructuring LHS (list_literal or
/// array_creation_expression) and define each as a local variable.
pub fn define_destructuring_variables(
    node: tree_sitter::Node,
    source: &[u8],
    tracker: &mut LocalVariableTracker,
    local_counter: &mut u32,
) {
    for i in 0..node.named_child_count() {
        let child = match node.named_child(i) {
            Some(c) => c,
            None => continue,
        };
        match child.kind() {
            "variable_name" => {
                let name = crate::parser::cst::node_text(child, source).trim_start_matches('$');
                let range = node_to_scip_range(child);
                tracker.define_variable(name, range, local_counter);
            }
            "list_literal" | "array_creation_expression" => {
                define_destructuring_variables(child, source, tracker, local_counter);
            }
            "array_element_initializer" => {
                // list($a, $b) style -- children are the variable_names
                define_destructuring_variables(child, source, tracker, local_counter);
            }
            _ => {
                // Recurse into other nodes that might contain variables
                // (e.g., array elements with keys: [$key => $val])
                define_destructuring_variables(child, source, tracker, local_counter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_push_pop() {
        let mut tracker = LocalVariableTracker::new();
        assert!(!tracker.has_scope());

        tracker.enter_scope(ScopeKind::Function);
        assert!(tracker.has_scope());

        tracker.enter_scope(ScopeKind::Closure);
        assert_eq!(tracker.scope_stack.len(), 2);

        tracker.exit_scope();
        assert_eq!(tracker.scope_stack.len(), 1);

        tracker.exit_scope();
        assert!(!tracker.has_scope());
    }

    #[test]
    fn test_arrow_function_no_scope_push() {
        let mut tracker = LocalVariableTracker::new();
        tracker.enter_scope(ScopeKind::Function);
        let before = tracker.scope_stack.len();
        tracker.enter_arrow_function();
        assert_eq!(tracker.scope_stack.len(), before);
        tracker.exit_arrow_function();
        assert_eq!(tracker.scope_stack.len(), before);
    }

    #[test]
    fn test_nested_closures() {
        let mut tracker = LocalVariableTracker::new();
        tracker.enter_scope(ScopeKind::Function);
        tracker.enter_scope(ScopeKind::Closure);
        tracker.enter_scope(ScopeKind::Closure);
        assert_eq!(tracker.scope_stack.len(), 3);
        tracker.exit_scope();
        tracker.exit_scope();
        tracker.exit_scope();
        assert_eq!(tracker.scope_stack.len(), 0);
    }

    #[test]
    fn test_simple_assignment_definition() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);

        // First assignment -> definition
        tracker.define_variable("x", vec![2, 8, 2, 10], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 1);
        assert!(tracker.local_occurrences[0].is_definition);
        assert_eq!(tracker.local_occurrences[0].symbol, "local 0");

        // Second assignment -> reference
        tracker.define_variable("x", vec![3, 8, 3, 10], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 2);
        assert!(!tracker.local_occurrences[1].is_definition);
        assert_eq!(tracker.local_occurrences[1].symbol, "local 0");
    }

    #[test]
    fn test_this_not_tracked() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);

        tracker.define_variable("this", vec![1, 0, 1, 5], &mut counter);
        assert!(tracker.local_occurrences.is_empty());
        assert_eq!(counter, 0);
    }

    #[test]
    fn test_register_param_no_occurrence() {
        let mut tracker = LocalVariableTracker::new();
        tracker.enter_scope(ScopeKind::Function);

        // Register param -- no occurrence emitted
        tracker.register_param("name", 0);
        assert!(tracker.local_occurrences.is_empty());

        // Now defining same var is a reference (already known)
        let mut counter = 1u32;
        tracker.define_variable("name", vec![3, 8, 3, 13], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 1);
        assert!(!tracker.local_occurrences[0].is_definition);
        assert_eq!(tracker.local_occurrences[0].symbol, "local 0");
    }

    #[test]
    fn test_separate_scopes_independent() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // First scope: define $x
        tracker.enter_scope(ScopeKind::Function);
        tracker.define_variable("x", vec![1, 0, 1, 2], &mut counter);
        assert!(tracker.local_occurrences[0].is_definition);
        tracker.exit_scope();

        // Second scope: define $x again -- should be a new definition
        tracker.enter_scope(ScopeKind::Closure);
        tracker.define_variable("x", vec![5, 0, 5, 2], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 2);
        assert!(tracker.local_occurrences[1].is_definition);
        // Different local ID
        assert_eq!(tracker.local_occurrences[0].symbol, "local 0");
        assert_eq!(tracker.local_occurrences[1].symbol, "local 1");
    }

    #[test]
    fn test_lookup_variable() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);
        tracker.define_variable("x", vec![1, 0, 1, 2], &mut counter);

        assert_eq!(tracker.lookup_variable("x"), Some(0));
        assert_eq!(tracker.lookup_variable("y"), None);
    }

    #[test]
    fn test_lookup_stops_at_scope_boundary() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        tracker.enter_scope(ScopeKind::Function);
        tracker.define_variable("x", vec![1, 0, 1, 2], &mut counter);

        // Closure scope: $x from outer function is NOT visible
        tracker.enter_scope(ScopeKind::Closure);
        assert_eq!(tracker.lookup_variable("x"), None);
    }

    #[test]
    fn test_no_scope_define_ignored() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // No scope pushed -- define should be a no-op
        tracker.define_variable("x", vec![0, 0, 0, 2], &mut counter);
        assert!(tracker.local_occurrences.is_empty());
    }
}
