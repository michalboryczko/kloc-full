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

    /// Emit a reference occurrence for a variable use.
    /// `name`: variable name WITHOUT the leading `$`
    /// `range`: SCIP range [line, col, end_line, end_col]
    pub fn reference_variable(
        &mut self,
        name: &str,
        range: Vec<u32>,
        local_counter: &mut u32,
    ) {
        if name == "this" {
            return;
        }

        if self.scope_stack.is_empty() {
            return;
        }

        let symbol = match self.lookup_variable(name) {
            Some(id) => SymbolNamer::symbol_for_local_var(id),
            None => {
                // Variable used before definition -- allocate a new local ID
                // to match PHP behavior (PHP allows undefined variable usage).
                let id = *local_counter;
                *local_counter += 1;
                if let Some(scope) = self.scope_stack.last_mut() {
                    scope.definitions.insert(name.to_string(), id);
                }
                SymbolNamer::symbol_for_local_var(id)
            }
        };

        self.local_occurrences.push(LocalOccurrence {
            symbol,
            range,
            is_definition: false,
        });
    }

    /// Process a closure's `use` clause.
    ///
    /// For each captured variable, emit a reference to the parent scope's
    /// definition and register the variable in the current (closure) scope.
    /// Must be called AFTER `enter_scope(Closure)` but BEFORE processing the body.
    ///
    /// `use_vars`: list of (variable_name_without_$, range)
    pub fn process_use_clause(
        &mut self,
        use_vars: Vec<(String, Vec<u32>)>,
        local_counter: &mut u32,
    ) {
        for (name, range) in use_vars {
            // Look up in parent scope (second-to-last on stack)
            let parent_symbol = if self.scope_stack.len() >= 2 {
                self.scope_stack[self.scope_stack.len() - 2]
                    .definitions
                    .get(&name)
                    .map(|id| SymbolNamer::symbol_for_local_var(*id))
            } else {
                None
            };

            if let Some(sym) = parent_symbol {
                // Emit reference pointing to parent scope's definition
                self.local_occurrences.push(LocalOccurrence {
                    symbol: sym,
                    range,
                    is_definition: false,
                });
            }

            // Register in current (closure) scope so body references resolve
            let id = *local_counter;
            *local_counter += 1;
            if let Some(scope) = self.scope_stack.last_mut() {
                scope.definitions.insert(name, id);
            }
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

    // ========================================================================
    // Subtask 14.3: reference_variable tests
    // ========================================================================

    #[test]
    fn test_reference_variable_after_definition() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);

        // Define $x
        tracker.define_variable("x", vec![1, 0, 1, 2], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 1);
        assert!(tracker.local_occurrences[0].is_definition);

        // Reference $x
        tracker.reference_variable("x", vec![2, 0, 2, 2], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 2);
        assert!(!tracker.local_occurrences[1].is_definition);
        // Same symbol as definition
        assert_eq!(tracker.local_occurrences[0].symbol, tracker.local_occurrences[1].symbol);
    }

    #[test]
    fn test_reference_variable_before_definition() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);

        // Reference $x before any definition
        tracker.reference_variable("x", vec![1, 0, 1, 2], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 1);
        assert!(!tracker.local_occurrences[0].is_definition);
        assert_eq!(tracker.local_occurrences[0].symbol, "local 0");

        // Subsequent definition of $x should be a reference (already registered)
        tracker.define_variable("x", vec![2, 0, 2, 2], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 2);
        assert!(!tracker.local_occurrences[1].is_definition);
        // Same symbol
        assert_eq!(tracker.local_occurrences[0].symbol, tracker.local_occurrences[1].symbol);
    }

    #[test]
    fn test_reference_this_skipped() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;
        tracker.enter_scope(ScopeKind::Function);

        tracker.reference_variable("this", vec![1, 0, 1, 5], &mut counter);
        assert!(tracker.local_occurrences.is_empty());
        assert_eq!(counter, 0);
    }

    #[test]
    fn test_reference_variable_no_scope() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // No scope -- reference should be a no-op
        tracker.reference_variable("x", vec![1, 0, 1, 2], &mut counter);
        assert!(tracker.local_occurrences.is_empty());
    }

    #[test]
    fn test_reference_param_in_body() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 1u32; // Start at 1; param was registered with id 0
        tracker.enter_scope(ScopeKind::Function);

        // Register param (no occurrence emitted)
        tracker.register_param("name", 0);

        // Reference $name in body
        tracker.reference_variable("name", vec![2, 10, 2, 15], &mut counter);
        assert_eq!(tracker.local_occurrences.len(), 1);
        assert!(!tracker.local_occurrences[0].is_definition);
        assert_eq!(tracker.local_occurrences[0].symbol, "local 0");
    }

    // ========================================================================
    // Subtask 14.4: process_use_clause tests
    // ========================================================================

    #[test]
    fn test_process_use_clause_references_parent() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // Outer function scope: define $message
        tracker.enter_scope(ScopeKind::Function);
        tracker.define_variable("message", vec![2, 4, 2, 12], &mut counter);
        assert_eq!(counter, 1);

        // Enter closure scope
        tracker.enter_scope(ScopeKind::Closure);

        // Process use clause: use ($message)
        tracker.process_use_clause(
            vec![("message".to_string(), vec![3, 25, 3, 33])],
            &mut counter,
        );

        // Should have: 1 def ($message in outer) + 1 ref ($message in use clause)
        assert_eq!(tracker.local_occurrences.len(), 2);
        assert!(tracker.local_occurrences[0].is_definition);
        assert!(!tracker.local_occurrences[1].is_definition);
        // use clause ref points to parent's symbol
        assert_eq!(tracker.local_occurrences[1].symbol, "local 0");

        // $message should be known in the closure scope
        assert!(tracker.lookup_variable("message").is_some());
    }

    #[test]
    fn test_process_use_clause_unknown_parent_var() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // Outer scope: no $unknown defined
        tracker.enter_scope(ScopeKind::Function);
        tracker.enter_scope(ScopeKind::Closure);

        // Process use clause for a variable not defined in parent
        tracker.process_use_clause(
            vec![("unknown".to_string(), vec![3, 25, 3, 33])],
            &mut counter,
        );

        // No reference emitted to parent (since parent doesn't have it)
        // But variable is still registered in closure scope
        assert!(tracker.local_occurrences.is_empty());
        assert!(tracker.lookup_variable("unknown").is_some());
    }

    #[test]
    fn test_nested_closure_capture_chain() {
        let mut tracker = LocalVariableTracker::new();
        let mut counter = 0u32;

        // Outer function: define $x
        tracker.enter_scope(ScopeKind::Function);
        tracker.define_variable("x", vec![1, 4, 1, 6], &mut counter);

        // Closure 1: capture $x
        tracker.enter_scope(ScopeKind::Closure);
        tracker.process_use_clause(
            vec![("x".to_string(), vec![3, 25, 3, 27])],
            &mut counter,
        );

        // Closure 2: capture $x from closure 1
        tracker.enter_scope(ScopeKind::Closure);
        tracker.process_use_clause(
            vec![("x".to_string(), vec![5, 25, 5, 27])],
            &mut counter,
        );

        // Reference $x in closure 2 body
        tracker.reference_variable("x", vec![6, 10, 6, 12], &mut counter);

        // Expected:
        // 1. $x def in outer (local 0)
        // 2. $x ref in use clause of closure 1 (local 0)
        // 3. $x ref in use clause of closure 2 (local 1, the captured copy in closure 1)
        // 4. $x ref in closure 2 body (local 2, the captured copy in closure 2)
        assert_eq!(tracker.local_occurrences.len(), 4);
        assert!(tracker.local_occurrences[0].is_definition); // def
        assert!(!tracker.local_occurrences[1].is_definition); // ref to parent
        assert!(!tracker.local_occurrences[2].is_definition); // ref to closure1's copy
        assert!(!tracker.local_occurrences[3].is_definition); // ref in body
    }
}
