//! Scope tracking for PHP symbol resolution.
//!
//! Maintains a stack of scope frames (namespace, class, method, function,
//! closure, arrow function) to track the current symbol context during
//! tree traversal.

use super::namer::SymbolNamer;

/// The kind of a class-like declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Interface,
    Trait,
    Enum,
}

/// A single frame on the scope stack.
#[derive(Debug, Clone)]
pub enum ScopeFrame {
    Namespace { fqn: String },
    Class { fqn: String, kind: ClassKind },
    Method { name: String, is_static: bool },
    Function { fqn: String },
    Closure { index: u32 },
    ArrowFunction { index: u32 },
}

/// Stack-based scope tracker for nested PHP declarations.
///
/// Tracks the current namespace, class, method/function context, and
/// closure nesting for correct symbol generation.
pub struct ScopeStack {
    frames: Vec<ScopeFrame>,
    closure_counter: u32,
}

impl ScopeStack {
    /// Create an empty scope stack.
    pub fn new() -> Self {
        ScopeStack {
            frames: Vec::new(),
            closure_counter: 0,
        }
    }

    /// Push a namespace frame.
    pub fn push_namespace(&mut self, fqn: String) {
        self.frames.push(ScopeFrame::Namespace { fqn });
    }

    /// Push a class (or interface/trait/enum) frame.
    pub fn push_class(&mut self, fqn: String, kind: ClassKind) {
        self.frames.push(ScopeFrame::Class { fqn, kind });
    }

    /// Push a method frame. Resets the closure counter.
    pub fn push_method(&mut self, name: String, is_static: bool) {
        self.closure_counter = 0;
        self.frames.push(ScopeFrame::Method { name, is_static });
    }

    /// Push a function frame. Resets the closure counter.
    pub fn push_function(&mut self, fqn: String) {
        self.closure_counter = 0;
        self.frames.push(ScopeFrame::Function { fqn });
    }

    /// Push a closure frame. Uses and increments the closure counter.
    pub fn push_closure(&mut self) {
        let index = self.closure_counter;
        self.closure_counter += 1;
        self.frames.push(ScopeFrame::Closure { index });
    }

    /// Push an arrow function frame. Uses and increments the closure counter.
    pub fn push_arrow_function(&mut self) {
        let index = self.closure_counter;
        self.closure_counter += 1;
        self.frames.push(ScopeFrame::ArrowFunction { index });
    }

    /// Pop the top frame from the stack.
    pub fn pop(&mut self) {
        self.frames.pop();
    }

    /// Find the FQN of the current class (reverse search for a Class frame).
    pub fn current_class(&self) -> Option<&str> {
        for frame in self.frames.iter().rev() {
            if let ScopeFrame::Class { fqn, .. } = frame {
                return Some(fqn.as_str());
            }
        }
        None
    }

    /// Find the name of the current method (reverse search for a Method frame).
    pub fn current_method(&self) -> Option<&str> {
        for frame in self.frames.iter().rev() {
            if let ScopeFrame::Method { name, .. } = frame {
                return Some(name.as_str());
            }
        }
        None
    }

    /// Get the FQN of the current callable context.
    ///
    /// - Method: returns `"ClassName::methodName"`
    /// - Function: returns the function FQN
    /// - Closure: returns `"closure#N"`
    pub fn current_callable_fqn(&self) -> Option<String> {
        for frame in self.frames.iter().rev() {
            match frame {
                ScopeFrame::Method { name, .. } => {
                    if let Some(class_fqn) = self.current_class() {
                        return Some(format!("{}::{}", class_fqn, name));
                    }
                    return Some(name.clone());
                }
                ScopeFrame::Function { fqn, .. } => {
                    return Some(fqn.clone());
                }
                ScopeFrame::Closure { index, .. } => {
                    return Some(format!("closure#{}", index));
                }
                ScopeFrame::ArrowFunction { index, .. } => {
                    return Some(format!("closure#{}", index));
                }
                _ => {}
            }
        }
        None
    }

    /// Check whether the current context is static (inside a static method).
    pub fn is_static_context(&self) -> bool {
        for frame in self.frames.iter().rev() {
            if let ScopeFrame::Method { is_static, .. } = frame {
                return *is_static;
            }
        }
        false
    }

    /// Get the current namespace FQN.
    pub fn current_namespace(&self) -> Option<&str> {
        for frame in self.frames.iter().rev() {
            if let ScopeFrame::Namespace { fqn, .. } = frame {
                return Some(fqn.as_str());
            }
        }
        None
    }

    /// Check if the cursor is directly inside a class body (no method/function after the class).
    ///
    /// Returns `true` if the last Class frame has no Method or Function frame after it.
    pub fn in_class_body(&self) -> bool {
        let mut found_class = false;
        for frame in self.frames.iter().rev() {
            match frame {
                ScopeFrame::Method { .. } | ScopeFrame::Function { .. } => {
                    // There's a method/function on top of the class
                    return false;
                }
                ScopeFrame::Class { .. } => {
                    found_class = true;
                    break;
                }
                _ => {}
            }
        }
        found_class
    }

    /// Get the SCIP class descriptor for the current class, if any.
    ///
    /// Uses `SymbolNamer::class_descriptor` on the current class FQN.
    pub fn current_class_descriptor(&self) -> Option<String> {
        self.current_class()
            .map(SymbolNamer::class_descriptor)
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_method_nesting() {
        let mut stack = ScopeStack::new();
        stack.push_namespace("App\\Models".to_string());
        stack.push_class("App\\Models\\User".to_string(), ClassKind::Class);
        stack.push_method("getName".to_string(), false);

        assert_eq!(stack.current_class(), Some("App\\Models\\User"));
        assert_eq!(stack.current_method(), Some("getName"));
        assert_eq!(
            stack.current_callable_fqn(),
            Some("App\\Models\\User::getName".to_string())
        );
    }

    #[test]
    fn test_pop_returns_to_class() {
        let mut stack = ScopeStack::new();
        stack.push_namespace("App\\Models".to_string());
        stack.push_class("App\\Models\\User".to_string(), ClassKind::Class);
        stack.push_method("getName".to_string(), false);

        // Pop method
        stack.pop();

        assert_eq!(stack.current_class(), Some("App\\Models\\User"));
        assert_eq!(stack.current_method(), None);
    }

    #[test]
    fn test_closure_in_method() {
        let mut stack = ScopeStack::new();
        stack.push_namespace("App".to_string());
        stack.push_class("App\\Foo".to_string(), ClassKind::Class);
        stack.push_method("bar".to_string(), false);
        stack.push_closure();

        // Class should still be accessible through closures
        assert_eq!(stack.current_class(), Some("App\\Foo"));
        // current_callable_fqn should return the closure
        assert_eq!(stack.current_callable_fqn(), Some("closure#0".to_string()));
    }

    #[test]
    fn test_static_method() {
        let mut stack = ScopeStack::new();
        stack.push_class("App\\Config".to_string(), ClassKind::Class);
        stack.push_method("getInstance".to_string(), true);

        assert!(stack.is_static_context());
    }

    #[test]
    fn test_no_class_returns_none() {
        let mut stack = ScopeStack::new();
        stack.push_namespace("App\\Helpers".to_string());
        stack.push_function("App\\Helpers\\format_date".to_string());

        assert_eq!(stack.current_class(), None);
        assert_eq!(
            stack.current_callable_fqn(),
            Some("App\\Helpers\\format_date".to_string())
        );
    }

    #[test]
    fn test_in_class_body() {
        let mut stack = ScopeStack::new();
        stack.push_namespace("App".to_string());
        stack.push_class("App\\Foo".to_string(), ClassKind::Class);

        // Directly in class body — no method yet
        assert!(stack.in_class_body());

        stack.push_method("bar".to_string(), false);
        // Now inside a method, not directly in class body
        assert!(!stack.in_class_body());

        stack.pop(); // pop method
        assert!(stack.in_class_body());
    }

    #[test]
    fn test_multiple_closures_sequential_indices() {
        let mut stack = ScopeStack::new();
        stack.push_method("doStuff".to_string(), false);

        stack.push_closure();
        assert_eq!(stack.current_callable_fqn(), Some("closure#0".to_string()));
        stack.pop();

        stack.push_closure();
        assert_eq!(stack.current_callable_fqn(), Some("closure#1".to_string()));
        stack.pop();

        stack.push_arrow_function();
        assert_eq!(stack.current_callable_fqn(), Some("closure#2".to_string()));
        stack.pop();
    }

    #[test]
    fn test_current_namespace() {
        let mut stack = ScopeStack::new();
        assert_eq!(stack.current_namespace(), None);

        stack.push_namespace("App\\Models".to_string());
        assert_eq!(stack.current_namespace(), Some("App\\Models"));
    }

    #[test]
    fn test_current_class_descriptor() {
        let mut stack = ScopeStack::new();
        assert_eq!(stack.current_class_descriptor(), None);

        stack.push_class("App\\Models\\User".to_string(), ClassKind::Class);
        assert_eq!(
            stack.current_class_descriptor(),
            Some("App/Models/User#".to_string())
        );
    }

    #[test]
    fn test_non_static_method() {
        let mut stack = ScopeStack::new();
        stack.push_class("App\\Foo".to_string(), ClassKind::Class);
        stack.push_method("bar".to_string(), false);

        assert!(!stack.is_static_context());
    }

    #[test]
    fn test_closure_counter_resets_on_method() {
        let mut stack = ScopeStack::new();
        stack.push_method("first".to_string(), false);
        stack.push_closure();
        assert_eq!(stack.current_callable_fqn(), Some("closure#0".to_string()));
        stack.pop();
        stack.push_closure();
        assert_eq!(stack.current_callable_fqn(), Some("closure#1".to_string()));
        stack.pop();
        stack.pop(); // pop method "first"

        // New method should reset the closure counter
        stack.push_method("second".to_string(), false);
        stack.push_closure();
        assert_eq!(stack.current_callable_fqn(), Some("closure#0".to_string()));
    }

    #[test]
    fn test_class_kind_variants() {
        let mut stack = ScopeStack::new();

        stack.push_class("App\\MyInterface".to_string(), ClassKind::Interface);
        assert_eq!(stack.current_class(), Some("App\\MyInterface"));
        stack.pop();

        stack.push_class("App\\MyTrait".to_string(), ClassKind::Trait);
        assert_eq!(stack.current_class(), Some("App\\MyTrait"));
        stack.pop();

        stack.push_class("App\\MyEnum".to_string(), ClassKind::Enum);
        assert_eq!(stack.current_class(), Some("App\\MyEnum"));
    }

    #[test]
    fn test_default_scope_stack() {
        let stack = ScopeStack::default();
        assert_eq!(stack.current_class(), None);
        assert_eq!(stack.current_method(), None);
        assert_eq!(stack.current_namespace(), None);
        assert_eq!(stack.current_callable_fqn(), None);
        assert!(!stack.is_static_context());
        assert!(!stack.in_class_body());
    }
}
