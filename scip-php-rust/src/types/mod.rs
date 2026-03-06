//! Type collection (Pass 1) and type resolution (Pass 2).
//!
//! Builds a global `TypeDatabase` containing all class/interface/trait/enum definitions,
//! their methods, properties, inheritance chains, and transitive upper (parent/interface) chains.

pub mod collector;
pub mod debug_dump;
pub mod phpdoc;
pub mod upper_chain;

use std::collections::HashMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════════
// SymbolKind
// ═══════════════════════════════════════════════════════════════════════════════

/// The kind of a top-level PHP type or symbol definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Class,
    AbstractClass,
    FinalClass,
    Interface,
    Trait,
    Enum,
    Function,
    Constant,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Visibility
// ═══════════════════════════════════════════════════════════════════════════════

/// PHP member visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TypeDef
// ═══════════════════════════════════════════════════════════════════════════════

/// A type definition (class, interface, trait, enum, or function).
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub kind: SymbolKind,
    pub file_path: PathBuf,
    pub is_abstract: bool,
    pub is_final: bool,
    pub is_readonly: bool,
    pub enum_backing_type: Option<String>,
    pub docblock: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ParamInfo
// ═══════════════════════════════════════════════════════════════════════════════

/// Information about a method or function parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub type_hint: Option<String>,
    pub has_default: bool,
    pub is_variadic: bool,
    pub is_promoted: bool,
    pub is_readonly: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TypeDatabase
// ═══════════════════════════════════════════════════════════════════════════════

/// Global type database populated during Pass 1 (type collection).
///
/// Keys are fully qualified names (FQNs) without a leading backslash.
/// Method/property keys use the format `"ClassName::methodName"` or `"ClassName::$propName"`.
pub struct TypeDatabase {
    /// Type definitions keyed by FQN.
    pub defs: HashMap<String, TypeDef>,
    /// Direct parent/interface/trait FQNs for each class-like type.
    pub uppers: HashMap<String, Vec<String>>,
    /// Method parameters keyed by `"Class::method"`.
    pub method_params: HashMap<String, Vec<ParamInfo>>,
    /// Property types keyed by `"Class::$prop"`.
    pub property_types: HashMap<String, Option<String>>,
    /// Method return types keyed by `"Class::method"`.
    pub method_return_types: HashMap<String, Option<String>>,
    /// Standalone function return types keyed by FQN.
    pub function_return_types: HashMap<String, Option<String>>,
    /// Transitive upper chain (all ancestors) keyed by FQN. Built by `upper_chain::build_transitive_uppers`.
    pub transitive_uppers: HashMap<String, Vec<String>>,
}

impl TypeDatabase {
    /// Create an empty TypeDatabase.
    pub fn new() -> Self {
        TypeDatabase {
            defs: HashMap::new(),
            uppers: HashMap::new(),
            method_params: HashMap::new(),
            property_types: HashMap::new(),
            method_return_types: HashMap::new(),
            function_return_types: HashMap::new(),
            transitive_uppers: HashMap::new(),
        }
    }

    /// Create a TypeDatabase with pre-allocated capacity.
    pub fn with_capacity(classes: usize, methods: usize) -> Self {
        TypeDatabase {
            defs: HashMap::with_capacity(classes),
            uppers: HashMap::with_capacity(classes),
            method_params: HashMap::with_capacity(methods),
            property_types: HashMap::with_capacity(methods),
            method_return_types: HashMap::with_capacity(methods),
            function_return_types: HashMap::with_capacity(classes / 4),
            transitive_uppers: HashMap::with_capacity(classes),
        }
    }

    /// Normalize an FQN by stripping a leading backslash if present.
    fn normalize_fqn(fqn: &str) -> &str {
        fqn.strip_prefix('\\').unwrap_or(fqn)
    }

    /// Insert a type definition. Returns `false` if a definition with this FQN already exists
    /// (first-write-wins semantics).
    pub fn insert_def(&mut self, fqn: &str, def: TypeDef) -> bool {
        let key = Self::normalize_fqn(fqn).to_string();
        if self.defs.contains_key(&key) {
            return false;
        }
        self.defs.insert(key, def);
        true
    }

    /// Look up a type definition by FQN.
    pub fn get_def(&self, fqn: &str) -> Option<&TypeDef> {
        self.defs.get(Self::normalize_fqn(fqn))
    }

    /// Check if a class-like type exists in the database.
    pub fn has_class(&self, fqn: &str) -> bool {
        self.defs.contains_key(Self::normalize_fqn(fqn))
    }

    /// Get the direct parents/interfaces/traits for a type.
    pub fn get_direct_uppers(&self, fqn: &str) -> &[String] {
        self.uppers
            .get(Self::normalize_fqn(fqn))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the transitive upper chain (all ancestors). Must call
    /// `upper_chain::build_transitive_uppers` first.
    pub fn get_all_uppers(&self, fqn: &str) -> &[String] {
        self.transitive_uppers
            .get(Self::normalize_fqn(fqn))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get method parameters by class FQN and method name.
    pub fn get_method_params(&self, class_fqn: &str, method_name: &str) -> Option<&[ParamInfo]> {
        let key = format!("{}::{}", Self::normalize_fqn(class_fqn), method_name);
        self.method_params.get(&key).map(|v| v.as_slice())
    }

    /// Get method return type by class FQN and method name.
    pub fn get_method_return_type(&self, class_fqn: &str, method_name: &str) -> Option<&str> {
        let key = format!("{}::{}", Self::normalize_fqn(class_fqn), method_name);
        self.method_return_types
            .get(&key)
            .and_then(|opt| opt.as_deref())
    }

    /// Get property type by class FQN and property name (without `$`).
    pub fn get_property_type(&self, class_fqn: &str, prop_name: &str) -> Option<&str> {
        let key = format!("{}::${}", Self::normalize_fqn(class_fqn), prop_name);
        self.property_types
            .get(&key)
            .and_then(|opt| opt.as_deref())
    }

    /// Add a method to a class.
    pub fn add_method(
        &mut self,
        class_fqn: &str,
        method_name: &str,
        return_type: Option<String>,
        params: Vec<ParamInfo>,
    ) {
        let key = format!("{}::{}", Self::normalize_fqn(class_fqn), method_name);
        self.method_params.insert(key.clone(), params);
        self.method_return_types.insert(key, return_type);
    }

    /// Add a property to a class. `prop_name` should NOT include the `$` prefix.
    pub fn add_property(
        &mut self,
        class_fqn: &str,
        prop_name: &str,
        type_hint: Option<String>,
    ) {
        let key = format!("{}::${}", Self::normalize_fqn(class_fqn), prop_name);
        self.property_types.insert(key, type_hint);
    }

    /// Set the direct parent/interface/trait FQNs for a type.
    pub fn add_uppers(&mut self, fqn: &str, parents: Vec<String>) {
        let key = Self::normalize_fqn(fqn).to_string();
        self.uppers.insert(key, parents);
    }

    /// Resolve which class defines a method by walking the transitive upper chain.
    /// Returns the FQN of the defining class, or `None` if the method is not found.
    pub fn resolve_method(&self, class_fqn: &str, method_name: &str) -> Option<String> {
        let fqn = Self::normalize_fqn(class_fqn);

        // Check the class itself first
        let key = format!("{}::{}", fqn, method_name);
        if self.method_params.contains_key(&key) {
            return Some(fqn.to_string());
        }

        // Walk transitive uppers
        for upper in self.get_all_uppers(fqn) {
            let upper_key = format!("{}::{}", upper, method_name);
            if self.method_params.contains_key(&upper_key) {
                return Some(upper.clone());
            }
        }

        None
    }

    /// Resolve which class defines a property by walking the transitive upper chain.
    /// Returns the FQN of the defining class, or `None` if the property is not found.
    pub fn resolve_property(&self, class_fqn: &str, prop_name: &str) -> Option<String> {
        let fqn = Self::normalize_fqn(class_fqn);

        // Check the class itself first
        let key = format!("{}::${}", fqn, prop_name);
        if self.property_types.contains_key(&key) {
            return Some(fqn.to_string());
        }

        // Walk transitive uppers
        for upper in self.get_all_uppers(fqn) {
            let upper_key = format!("{}::${}", upper, prop_name);
            if self.property_types.contains_key(&upper_key) {
                return Some(upper.clone());
            }
        }

        None
    }

    /// Resolve method return type by walking the transitive upper chain.
    /// Returns the return type from the first class that defines the method.
    pub fn resolve_method_return_type(&self, class_fqn: &str, method_name: &str) -> Option<&str> {
        let fqn = Self::normalize_fqn(class_fqn);

        // Check the class itself first
        if let Some(rt) = self.get_method_return_type(fqn, method_name) {
            return Some(rt);
        }

        // Walk transitive uppers
        for upper in self.get_all_uppers(fqn) {
            if let Some(rt) = self.get_method_return_type(upper, method_name) {
                return Some(rt);
            }
        }

        None
    }
}

impl Default for TypeDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_class_def(kind: SymbolKind) -> TypeDef {
        TypeDef {
            kind,
            file_path: PathBuf::from("test.php"),
            is_abstract: false,
            is_final: false,
            is_readonly: false,
            enum_backing_type: None,
            docblock: None,
        }
    }

    #[test]
    fn test_insert_and_lookup() {
        let mut db = TypeDatabase::new();
        let def = make_class_def(SymbolKind::Class);
        assert!(db.insert_def("App\\Models\\User", def));
        assert!(db.has_class("App\\Models\\User"));
        assert!(!db.has_class("App\\Models\\Post"));

        let retrieved = db.get_def("App\\Models\\User").unwrap();
        assert_eq!(retrieved.kind, SymbolKind::Class);
    }

    #[test]
    fn test_strip_leading_backslash() {
        let mut db = TypeDatabase::new();
        let def = make_class_def(SymbolKind::Interface);
        db.insert_def("\\Serializable", def);

        // Should be accessible with or without leading backslash
        assert!(db.has_class("Serializable"));
        assert!(db.has_class("\\Serializable"));
        assert!(db.get_def("Serializable").is_some());
        assert!(db.get_def("\\Serializable").is_some());
    }

    #[test]
    fn test_duplicate_detection() {
        let mut db = TypeDatabase::new();
        let def1 = TypeDef {
            kind: SymbolKind::Class,
            file_path: PathBuf::from("first.php"),
            is_abstract: false,
            is_final: false,
            is_readonly: false,
            enum_backing_type: None,
            docblock: None,
        };
        let def2 = TypeDef {
            kind: SymbolKind::Class,
            file_path: PathBuf::from("second.php"),
            is_abstract: false,
            is_final: false,
            is_readonly: false,
            enum_backing_type: None,
            docblock: None,
        };

        assert!(db.insert_def("App\\Foo", def1));
        assert!(!db.insert_def("App\\Foo", def2)); // duplicate

        // First-write-wins: file_path should be first.php
        let def = db.get_def("App\\Foo").unwrap();
        assert_eq!(def.file_path, PathBuf::from("first.php"));
    }

    #[test]
    fn test_method_add_and_lookup() {
        let mut db = TypeDatabase::new();
        db.insert_def("App\\User", make_class_def(SymbolKind::Class));

        let params = vec![
            ParamInfo {
                name: "name".to_string(),
                type_hint: Some("string".to_string()),
                has_default: false,
                is_variadic: false,
                is_promoted: false,
                is_readonly: false,
            },
            ParamInfo {
                name: "age".to_string(),
                type_hint: Some("int".to_string()),
                has_default: true,
                is_variadic: false,
                is_promoted: false,
                is_readonly: false,
            },
        ];

        db.add_method("App\\User", "setProfile", Some("void".to_string()), params);

        let retrieved_params = db.get_method_params("App\\User", "setProfile").unwrap();
        assert_eq!(retrieved_params.len(), 2);
        assert_eq!(retrieved_params[0].name, "name");
        assert_eq!(retrieved_params[1].has_default, true);

        let rt = db.get_method_return_type("App\\User", "setProfile").unwrap();
        assert_eq!(rt, "void");
    }

    #[test]
    fn test_property_add_and_lookup() {
        let mut db = TypeDatabase::new();
        db.insert_def("App\\User", make_class_def(SymbolKind::Class));

        db.add_property("App\\User", "name", Some("string".to_string()));
        db.add_property("App\\User", "meta", None);

        assert_eq!(
            db.get_property_type("App\\User", "name"),
            Some("string")
        );
        assert_eq!(db.get_property_type("App\\User", "meta"), None);
    }

    #[test]
    fn test_uppers() {
        let mut db = TypeDatabase::new();
        db.insert_def("App\\User", make_class_def(SymbolKind::Class));
        db.add_uppers(
            "App\\User",
            vec![
                "App\\Base\\Model".to_string(),
                "App\\Contracts\\HasRole".to_string(),
            ],
        );

        let uppers = db.get_direct_uppers("App\\User");
        assert_eq!(uppers.len(), 2);
        assert_eq!(uppers[0], "App\\Base\\Model");
        assert_eq!(uppers[1], "App\\Contracts\\HasRole");
    }

    #[test]
    fn test_method_resolution_through_upper_chain() {
        let mut db = TypeDatabase::new();

        // Set up: Child extends Parent extends GrandParent
        db.insert_def("GrandParent", make_class_def(SymbolKind::Class));
        db.insert_def("Parent", make_class_def(SymbolKind::Class));
        db.insert_def("Child", make_class_def(SymbolKind::Class));

        db.add_uppers("Parent", vec!["GrandParent".to_string()]);
        db.add_uppers("Child", vec!["Parent".to_string()]);

        // GrandParent defines "baseMethod"
        db.add_method("GrandParent", "baseMethod", Some("void".to_string()), vec![]);
        // Parent defines "parentMethod"
        db.add_method("Parent", "parentMethod", Some("string".to_string()), vec![]);
        // Child defines "childMethod"
        db.add_method("Child", "childMethod", Some("int".to_string()), vec![]);

        // Build transitive uppers
        upper_chain::build_transitive_uppers(&mut db);

        // Child can resolve all three
        assert_eq!(
            db.resolve_method("Child", "childMethod"),
            Some("Child".to_string())
        );
        assert_eq!(
            db.resolve_method("Child", "parentMethod"),
            Some("Parent".to_string())
        );
        assert_eq!(
            db.resolve_method("Child", "baseMethod"),
            Some("GrandParent".to_string())
        );

        // Non-existent method
        assert_eq!(db.resolve_method("Child", "nonExistent"), None);
    }

    #[test]
    fn test_property_resolution_through_upper_chain() {
        let mut db = TypeDatabase::new();

        db.insert_def("Base", make_class_def(SymbolKind::Class));
        db.insert_def("Child", make_class_def(SymbolKind::Class));
        db.add_uppers("Child", vec!["Base".to_string()]);

        db.add_property("Base", "baseProp", Some("string".to_string()));
        db.add_property("Child", "childProp", Some("int".to_string()));

        upper_chain::build_transitive_uppers(&mut db);

        assert_eq!(
            db.resolve_property("Child", "childProp"),
            Some("Child".to_string())
        );
        assert_eq!(
            db.resolve_property("Child", "baseProp"),
            Some("Base".to_string())
        );
        assert_eq!(db.resolve_property("Child", "nope"), None);
    }

    #[test]
    fn test_resolve_method_return_type_through_chain() {
        let mut db = TypeDatabase::new();

        db.insert_def("Base", make_class_def(SymbolKind::Class));
        db.insert_def("Child", make_class_def(SymbolKind::Class));
        db.add_uppers("Child", vec!["Base".to_string()]);

        db.add_method("Base", "getName", Some("string".to_string()), vec![]);

        upper_chain::build_transitive_uppers(&mut db);

        // Child doesn't define getName, but Base does
        assert_eq!(
            db.resolve_method_return_type("Child", "getName"),
            Some("string")
        );
    }

    #[test]
    fn test_with_capacity() {
        let db = TypeDatabase::with_capacity(100, 500);
        assert!(db.defs.capacity() >= 100);
        assert!(db.method_params.capacity() >= 500);
    }
}
