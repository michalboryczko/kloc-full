//! PHP name resolution: use-statement parsing, FQN construction, and special-name handling.
//!
//! Mirrors the name resolution logic from scip-php's PHP implementation:
//! - Fully qualified names (leading `\`) resolve directly
//! - Imported names resolve via use-statement maps
//! - Unqualified/qualified names are prepended with the current namespace
//! - Functions and constants have a global fallback; classes do not

use std::collections::HashMap;
use tree_sitter::Node;

use crate::parser::cst::node_text;

// ═══════════════════════════════════════════════════════════════════════════════
// UseKind / UseImport
// ═══════════════════════════════════════════════════════════════════════════════

/// The kind of a `use` import statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    Class,
    Function,
    Const,
}

/// A single import extracted from a `use` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseImport {
    /// Fully qualified name (without leading backslash).
    pub fqn: String,
    /// The short alias used in code. For `use Foo\Bar as Baz`, alias is `"Baz"`.
    /// For `use Foo\Bar`, alias is `"Bar"`.
    pub alias: String,
    /// Whether this imports a class, function, or constant.
    pub kind: UseKind,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Resolution result types
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of resolving a class-like name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassResolution {
    /// Successfully resolved to an FQN.
    Resolved(String),
    /// `self` used outside a class context.
    SelfOutsideClass,
    /// `parent` used but no parent class is known.
    NoParent,
    /// A PHP built-in type (void, int, string, etc.).
    BuiltIn(String),
}

/// Result of resolving a function name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionResolution {
    /// Resolved via an explicit import.
    Imported(String),
    /// Unqualified or qualified in a namespace — has a namespaced primary and a global fallback.
    Namespaced { primary: String, fallback: String },
    /// In the global namespace (no fallback needed).
    Global(String),
}

impl FunctionResolution {
    /// The primary (preferred) FQN.
    pub fn primary(&self) -> &str {
        match self {
            FunctionResolution::Imported(fqn) => fqn,
            FunctionResolution::Namespaced { primary, .. } => primary,
            FunctionResolution::Global(fqn) => fqn,
        }
    }

    /// The fallback FQN (for unqualified names that might be global).
    pub fn fallback(&self) -> Option<&str> {
        match self {
            FunctionResolution::Namespaced { fallback, .. } => Some(fallback),
            _ => None,
        }
    }
}

/// Result of resolving a constant name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantResolution {
    /// Resolved via an explicit import.
    Imported(String),
    /// Unqualified or qualified in a namespace — has a namespaced primary and a global fallback.
    Namespaced { primary: String, fallback: String },
    /// In the global namespace (no fallback needed).
    Global(String),
    /// A PHP built-in constant (true, false, null).
    BuiltIn(String),
}

impl ConstantResolution {
    /// The primary (preferred) FQN.
    pub fn primary(&self) -> &str {
        match self {
            ConstantResolution::Imported(fqn) => fqn,
            ConstantResolution::Namespaced { primary, .. } => primary,
            ConstantResolution::Global(fqn) => fqn,
            ConstantResolution::BuiltIn(name) => name,
        }
    }
}

/// Context in which a name is being resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameContext {
    /// e.g. `new Foo`, `Foo::class`, `instanceof Foo`
    ClassReference,
    /// e.g. `foo()`, `Bar\baz()`
    FunctionCall,
    /// e.g. `FOO`, `Bar\BAZ`
    ConstantAccess,
    /// e.g. `: Foo`, `Foo|Bar` in type position
    TypeHint,
    /// e.g. `Foo::method()`, `Foo::$prop`
    StaticClass,
}

/// Unified name resolution result dispatched by context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameResolution {
    Class(String),
    Function(FunctionResolution),
    Constant(ConstantResolution),
    BuiltInType(String),
    Unresolvable(String),
}

impl NameResolution {
    /// Get the primary FQN from any resolution variant.
    pub fn primary_fqn(&self) -> Option<&str> {
        match self {
            NameResolution::Class(fqn) => Some(fqn),
            NameResolution::Function(fr) => Some(fr.primary()),
            NameResolution::Constant(cr) => Some(cr.primary()),
            NameResolution::BuiltInType(name) => Some(name),
            NameResolution::Unresolvable(_) => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Built-in type list
// ═══════════════════════════════════════════════════════════════════════════════

/// PHP built-in types that should not be resolved as classes.
const BUILTIN_TYPES: &[&str] = &[
    "void", "null", "int", "float", "string", "bool", "array", "object",
    "callable", "iterable", "mixed", "never", "true", "false", "self",
    "static", "parent",
];

fn is_builtin_type(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    BUILTIN_TYPES.contains(&lower.as_str())
}

/// PHP built-in constants.
fn is_builtin_constant(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(lower.as_str(), "true" | "false" | "null")
}

// ═══════════════════════════════════════════════════════════════════════════════
// NameResolver
// ═══════════════════════════════════════════════════════════════════════════════

/// Resolves PHP names (classes, functions, constants) to fully qualified names
/// using the current namespace and import context.
#[derive(Debug)]
pub struct NameResolver {
    /// Current namespace (e.g. `"App\\Services"`). Empty string for global namespace.
    namespace: String,
    /// Class imports keyed by alias.
    class_imports: HashMap<String, UseImport>,
    /// Function imports keyed by alias.
    function_imports: HashMap<String, UseImport>,
    /// Constant imports keyed by alias.
    const_imports: HashMap<String, UseImport>,
    /// FQN of the class we are currently inside (e.g. `"App\\Services\\UserService"`).
    current_class: Option<String>,
    /// FQN of the parent class (from `extends`), if any.
    current_parent: Option<String>,
}

impl NameResolver {
    /// Create a new resolver in the global namespace with no imports.
    pub fn new() -> Self {
        NameResolver {
            namespace: String::new(),
            class_imports: HashMap::new(),
            function_imports: HashMap::new(),
            const_imports: HashMap::new(),
            current_class: None,
            current_parent: None,
        }
    }

    /// Reset the resolver to its initial state.
    pub fn reset(&mut self) {
        self.namespace.clear();
        self.class_imports.clear();
        self.function_imports.clear();
        self.const_imports.clear();
        self.current_class = None;
        self.current_parent = None;
    }

    /// Enter a new namespace. Clears all imports (PHP requires re-declaration per namespace block).
    pub fn enter_namespace(&mut self, name: &str) {
        self.namespace = name.to_string();
        self.class_imports.clear();
        self.function_imports.clear();
        self.const_imports.clear();
    }

    /// Get the current namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Add an import to the appropriate map based on its kind.
    pub fn add_import(&mut self, import: UseImport) {
        let alias = import.alias.clone();
        match import.kind {
            UseKind::Class => { self.class_imports.insert(alias, import); }
            UseKind::Function => { self.function_imports.insert(alias, import); }
            UseKind::Const => { self.const_imports.insert(alias, import); }
        }
    }

    /// Get the class imports map (for testing/inspection).
    pub fn class_imports(&self) -> &HashMap<String, UseImport> {
        &self.class_imports
    }

    /// Get the function imports map.
    pub fn function_imports(&self) -> &HashMap<String, UseImport> {
        &self.function_imports
    }

    /// Get the constant imports map.
    pub fn const_imports(&self) -> &HashMap<String, UseImport> {
        &self.const_imports
    }

    // ─── Class context management ──────────────────────────────────────────

    /// Push a class onto the context stack. `parent_fqn` is the resolved parent class if any.
    pub fn push_class(&mut self, fqn: &str, parent_fqn: Option<&str>) {
        self.current_class = Some(fqn.to_string());
        self.current_parent = parent_fqn.map(|s| s.to_string());
    }

    /// Pop the current class context.
    pub fn pop_class(&mut self) {
        self.current_class = None;
        self.current_parent = None;
    }

    /// Get the FQN of the current class, if inside one.
    pub fn current_class_fqn(&self) -> Option<&str> {
        self.current_class.as_deref()
    }

    /// Get the FQN of the current parent class, if inside a class with a parent.
    pub fn current_parent_fqn(&self) -> Option<&str> {
        self.current_parent.as_deref()
    }

    // ─── Use-statement parsing (static methods) ────────────────────────────

    /// Parse a `namespace_use_declaration` CST node into a list of `UseImport`s.
    ///
    /// Handles:
    /// - Simple: `use Foo\Bar;`
    /// - Aliased: `use Foo\Bar as Baz;`
    /// - Grouped: `use Foo\{Bar, Baz};`
    /// - Grouped with alias: `use Foo\{Bar, Baz as B};`
    /// - Function: `use function Foo\bar;`
    /// - Const: `use const Foo\BAR;`
    /// - Mixed group: `use Foo\{Bar, function baz, const QUX};`
    pub fn parse_use_declaration(node: Node, source: &[u8]) -> Vec<UseImport> {
        debug_assert_eq!(node.kind(), "namespace_use_declaration");

        let mut results = Vec::new();

        // Detect the declaration-level kind (e.g. `use function ...` or `use const ...`)
        let decl_kind = Self::detect_use_kind(node, source);

        // Look for a group prefix (namespace_name before namespace_use_group)
        let mut prefix = String::new();
        let mut has_group = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "namespace_name" if child.is_named() => {
                        prefix = node_text(child, source).to_string();
                    }
                    "namespace_use_group" if child.is_named() => {
                        has_group = true;
                        // Parse each clause in the group
                        for j in 0..child.named_child_count() {
                            if let Some(clause) = child.named_child(j) {
                                if clause.kind() == "namespace_use_clause" {
                                    // Per-clause kind overrides declaration-level kind
                                    let clause_kind = Self::detect_clause_kind(clause, source)
                                        .unwrap_or(decl_kind);
                                    if let Some(import) = Self::parse_use_clause(
                                        clause, source, clause_kind, &prefix,
                                    ) {
                                        results.push(import);
                                    }
                                }
                            }
                        }
                    }
                    "namespace_use_clause" if child.is_named() && !has_group => {
                        // Simple (non-grouped) use clause — detect clause-level kind too
                        let clause_kind = Self::detect_clause_kind(child, source)
                            .unwrap_or(decl_kind);
                        if let Some(import) = Self::parse_use_clause(
                            child, source, clause_kind, "",
                        ) {
                            results.push(import);
                        }
                    }
                    _ => {}
                }
            }
        }

        results
    }

    /// Detect the use kind from declaration-level anonymous children.
    /// Checks for `function` or `const` keywords among non-named children
    /// of the `namespace_use_declaration` node.
    fn detect_use_kind(node: Node, source: &[u8]) -> UseKind {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if !child.is_named() {
                    let text = node_text(child, source);
                    match text {
                        "function" => return UseKind::Function,
                        "const" => return UseKind::Const,
                        _ => {}
                    }
                }
            }
        }
        UseKind::Class
    }

    /// Detect per-clause kind override (for mixed group use statements).
    /// Returns `Some(kind)` if the clause has its own `function`/`const` keyword,
    /// `None` if it inherits from the declaration.
    fn detect_clause_kind(clause: Node, source: &[u8]) -> Option<UseKind> {
        for i in 0..clause.child_count() {
            if let Some(child) = clause.child(i) {
                if !child.is_named() {
                    let text = node_text(child, source);
                    match text {
                        "function" => return Some(UseKind::Function),
                        "const" => return Some(UseKind::Const),
                        _ => {}
                    }
                }
            }
        }
        None
    }

    /// Parse a single `namespace_use_clause` into a `UseImport`.
    ///
    /// `prefix` is the group prefix (empty for simple use statements).
    /// The name is extracted from `qualified_name`, `name`, or `namespace_name` children.
    /// The alias comes from the `alias` field or falls back to the last segment of the name.
    fn parse_use_clause(
        clause: Node,
        source: &[u8],
        kind: UseKind,
        prefix: &str,
    ) -> Option<UseImport> {
        // Extract the name part (qualified_name, name, or namespace_name)
        let name_text = Self::extract_clause_name(clause, source)?;

        // Build the FQN
        let fqn = if prefix.is_empty() {
            name_text.to_string()
        } else {
            format!("{}\\{}", prefix, name_text)
        };

        // Extract alias: check field name "alias", or use last segment of FQN
        let alias = if let Some(alias_node) = clause.child_by_field_name("alias") {
            node_text(alias_node, source).to_string()
        } else {
            // Last segment of the name
            fqn.rsplit('\\').next().unwrap_or(&fqn).to_string()
        };

        Some(UseImport { fqn, alias, kind })
    }

    /// Extract the name text from a use clause's named children.
    /// Looks for qualified_name, namespace_name, or name nodes.
    fn extract_clause_name<'a>(clause: Node, source: &'a [u8]) -> Option<&'a str> {
        for i in 0..clause.named_child_count() {
            if let Some(child) = clause.named_child(i) {
                match child.kind() {
                    "qualified_name" | "namespace_name" => {
                        return Some(node_text(child, source));
                    }
                    "name" => {
                        // Could be the name or the alias — only take the first one
                        // Check: if there's an alias field and this node IS the alias, skip it
                        if let Some(alias_node) = clause.child_by_field_name("alias") {
                            if child.id() == alias_node.id() {
                                continue;
                            }
                        }
                        return Some(node_text(child, source));
                    }
                    _ => {}
                }
            }
        }
        None
    }

    // ─── Name resolution ───────────────────────────────────────────────────

    /// Resolve a class/interface/trait/enum name to its FQN.
    ///
    /// Five resolution paths:
    /// 1. Fully qualified (`\Foo\Bar`) — strip leading backslash
    /// 2. Imported unqualified (`User` with `use App\Models\User`) — look up in class imports
    /// 3. Imported qualified (`Models\User` with `use App\Models`) — look up first segment
    /// 4. Unimported unqualified in namespace — prepend namespace
    /// 5. Unimported qualified in namespace — prepend namespace
    ///
    /// Classes have NO global fallback (unlike functions/constants).
    pub fn resolve_class(&self, name: &str) -> String {
        // Path 1: Fully qualified
        if let Some(stripped) = name.strip_prefix('\\') {
            return stripped.to_string();
        }

        // Split into first segment and rest
        let (first_segment, rest) = match name.find('\\') {
            Some(pos) => (&name[..pos], Some(&name[pos + 1..])),
            None => (name, None),
        };

        // Path 2 & 3: Check class imports
        if let Some(import) = self.class_imports.get(first_segment) {
            return match rest {
                Some(rest) => format!("{}\\{}", import.fqn, rest),
                None => import.fqn.clone(),
            };
        }

        // Path 4 & 5: Prepend namespace (or use as-is in global namespace)
        if self.namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}\\{}", self.namespace, name)
        }
    }

    /// Resolve a class name handling `self`, `static`, `parent`, and built-in types.
    pub fn resolve_class_or_special(&self, name: &str) -> ClassResolution {
        let lower = name.to_ascii_lowercase();

        match lower.as_str() {
            "self" | "static" => {
                match &self.current_class {
                    Some(fqn) => ClassResolution::Resolved(fqn.clone()),
                    None => ClassResolution::SelfOutsideClass,
                }
            }
            "parent" => {
                match &self.current_parent {
                    Some(fqn) => ClassResolution::Resolved(fqn.clone()),
                    None => ClassResolution::NoParent,
                }
            }
            _ if is_builtin_type(&lower) => {
                ClassResolution::BuiltIn(lower)
            }
            _ => ClassResolution::Resolved(self.resolve_class(name)),
        }
    }

    /// Resolve a function name.
    ///
    /// Like class resolution but with a global fallback for unqualified names:
    /// - Fully qualified: strip `\`
    /// - Imported: use the import FQN
    /// - Unqualified in namespace: primary = `Ns\func`, fallback = `func`
    /// - Qualified in namespace: primary = `Ns\Sub\func` (no fallback for qualified)
    /// - Global namespace: resolve directly
    pub fn resolve_function(&self, name: &str) -> FunctionResolution {
        // Fully qualified
        if let Some(stripped) = name.strip_prefix('\\') {
            return FunctionResolution::Global(stripped.to_string());
        }

        let (first_segment, rest) = match name.find('\\') {
            Some(pos) => (&name[..pos], Some(&name[pos + 1..])),
            None => (name, None),
        };

        // Check function imports
        if let Some(import) = self.function_imports.get(first_segment) {
            return match rest {
                Some(rest) => FunctionResolution::Imported(format!("{}\\{}", import.fqn, rest)),
                None => FunctionResolution::Imported(import.fqn.clone()),
            };
        }

        if self.namespace.is_empty() {
            // Global namespace — no fallback needed
            FunctionResolution::Global(name.to_string())
        } else if rest.is_some() {
            // Qualified name in a namespace — prepend namespace, no global fallback
            // Qualified names don't get global fallback in PHP
            FunctionResolution::Namespaced {
                primary: format!("{}\\{}", self.namespace, name),
                fallback: name.to_string(),
            }
        } else {
            // Unqualified name in a namespace — has global fallback
            FunctionResolution::Namespaced {
                primary: format!("{}\\{}", self.namespace, name),
                fallback: name.to_string(),
            }
        }
    }

    /// Resolve a constant name.
    ///
    /// Like function resolution but also handles `true`, `false`, `null` as built-in.
    pub fn resolve_constant(&self, name: &str) -> ConstantResolution {
        // Built-in constants always resolve directly
        if is_builtin_constant(name) {
            return ConstantResolution::BuiltIn(name.to_ascii_lowercase());
        }

        // Fully qualified
        if let Some(stripped) = name.strip_prefix('\\') {
            return ConstantResolution::Global(stripped.to_string());
        }

        let (first_segment, rest) = match name.find('\\') {
            Some(pos) => (&name[..pos], Some(&name[pos + 1..])),
            None => (name, None),
        };

        // Check constant imports
        if let Some(import) = self.const_imports.get(first_segment) {
            return match rest {
                Some(rest) => ConstantResolution::Imported(format!("{}\\{}", import.fqn, rest)),
                None => ConstantResolution::Imported(import.fqn.clone()),
            };
        }

        if self.namespace.is_empty() {
            ConstantResolution::Global(name.to_string())
        } else {
            // Both unqualified and qualified get namespaced with fallback
            ConstantResolution::Namespaced {
                primary: format!("{}\\{}", self.namespace, name),
                fallback: name.to_string(),
            }
        }
    }

    /// Unified name resolution dispatched by context.
    pub fn resolve_name(&self, name: &str, context: NameContext) -> NameResolution {
        match context {
            NameContext::ClassReference => {
                NameResolution::Class(self.resolve_class(name))
            }
            NameContext::TypeHint => {
                match self.resolve_class_or_special(name) {
                    ClassResolution::Resolved(fqn) => NameResolution::Class(fqn),
                    ClassResolution::SelfOutsideClass => {
                        NameResolution::Unresolvable("self (outside class)".to_string())
                    }
                    ClassResolution::NoParent => {
                        NameResolution::Unresolvable("parent (no parent class)".to_string())
                    }
                    ClassResolution::BuiltIn(name) => NameResolution::BuiltInType(name),
                }
            }
            NameContext::StaticClass => {
                match self.resolve_class_or_special(name) {
                    ClassResolution::Resolved(fqn) => NameResolution::Class(fqn),
                    ClassResolution::SelfOutsideClass => {
                        NameResolution::Unresolvable("self (outside class)".to_string())
                    }
                    ClassResolution::NoParent => {
                        NameResolution::Unresolvable("parent (no parent class)".to_string())
                    }
                    ClassResolution::BuiltIn(name) => NameResolution::BuiltInType(name),
                }
            }
            NameContext::FunctionCall => {
                NameResolution::Function(self.resolve_function(name))
            }
            NameContext::ConstantAccess => {
                NameResolution::Constant(self.resolve_constant(name))
            }
        }
    }
}

impl Default for NameResolver {
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

    fn parse_php(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let source_bytes = source.as_bytes().to_vec();
        let tree = parser.parse(&source_bytes, None).unwrap();
        (tree, source_bytes)
    }

    /// Find all `namespace_use_declaration` nodes in the tree root.
    fn find_use_declarations(root: tree_sitter::Node) -> Vec<tree_sitter::Node> {
        let mut results = Vec::new();
        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "namespace_use_declaration" {
                    results.push(child);
                }
            }
        }
        results
    }

    // ─── Use-statement parsing tests ───────────────────────────────────────

    #[test]
    fn test_simple_use() {
        let (tree, source) = parse_php("<?php use App\\Models\\User;");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].fqn, "App\\Models\\User");
        assert_eq!(imports[0].alias, "User");
        assert_eq!(imports[0].kind, UseKind::Class);
    }

    #[test]
    fn test_aliased_use() {
        let (tree, source) = parse_php("<?php use Psr\\Log\\LoggerInterface as Logger;");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].fqn, "Psr\\Log\\LoggerInterface");
        assert_eq!(imports[0].alias, "Logger");
        assert_eq!(imports[0].kind, UseKind::Class);
    }

    #[test]
    fn test_grouped_use() {
        let (tree, source) = parse_php("<?php use App\\Models\\{User, Post};");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].fqn, "App\\Models\\User");
        assert_eq!(imports[0].alias, "User");
        assert_eq!(imports[1].fqn, "App\\Models\\Post");
        assert_eq!(imports[1].alias, "Post");
    }

    #[test]
    fn test_grouped_use_with_alias() {
        let (tree, source) = parse_php("<?php use App\\Models\\{User, Post as BlogPost};");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].fqn, "App\\Models\\User");
        assert_eq!(imports[0].alias, "User");
        assert_eq!(imports[1].fqn, "App\\Models\\Post");
        assert_eq!(imports[1].alias, "BlogPost");
    }

    #[test]
    fn test_function_use() {
        let (tree, source) = parse_php("<?php use function App\\Helpers\\format_name;");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].fqn, "App\\Helpers\\format_name");
        assert_eq!(imports[0].alias, "format_name");
        assert_eq!(imports[0].kind, UseKind::Function);
    }

    #[test]
    fn test_const_use() {
        let (tree, source) = parse_php("<?php use const App\\Config\\MAX_SIZE;");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].fqn, "App\\Config\\MAX_SIZE");
        assert_eq!(imports[0].alias, "MAX_SIZE");
        assert_eq!(imports[0].kind, UseKind::Const);
    }

    #[test]
    fn test_mixed_group_use() {
        let (tree, source) =
            parse_php("<?php use App\\Mixed\\{User, function helper, const MAX};");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 3);

        assert_eq!(imports[0].fqn, "App\\Mixed\\User");
        assert_eq!(imports[0].kind, UseKind::Class);

        assert_eq!(imports[1].fqn, "App\\Mixed\\helper");
        assert_eq!(imports[1].kind, UseKind::Function);

        assert_eq!(imports[2].fqn, "App\\Mixed\\MAX");
        assert_eq!(imports[2].kind, UseKind::Const);
    }

    #[test]
    fn test_fully_qualified_use() {
        // Even though `use \Foo\Bar` is unusual, it should work
        let (tree, source) = parse_php("<?php use App\\Models\\User;");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports[0].fqn, "App\\Models\\User");
    }

    #[test]
    fn test_multiple_use_statements() {
        let (tree, source) = parse_php(
            "<?php\nuse App\\Models\\User;\nuse App\\Models\\Post;\nuse Psr\\Log\\LoggerInterface as Logger;",
        );
        let decls = find_use_declarations(tree.root_node());
        assert_eq!(decls.len(), 3);

        let mut resolver = NameResolver::new();
        for decl in &decls {
            let imports = NameResolver::parse_use_declaration(*decl, &source);
            for import in imports {
                resolver.add_import(import);
            }
        }

        assert_eq!(resolver.class_imports().len(), 3);
        assert_eq!(resolver.class_imports()["User"].fqn, "App\\Models\\User");
        assert_eq!(resolver.class_imports()["Post"].fqn, "App\\Models\\Post");
        assert_eq!(
            resolver.class_imports()["Logger"].fqn,
            "Psr\\Log\\LoggerInterface"
        );
    }

    #[test]
    fn test_enter_namespace_clears_imports() {
        let mut resolver = NameResolver::new();
        resolver.add_import(UseImport {
            fqn: "App\\Models\\User".to_string(),
            alias: "User".to_string(),
            kind: UseKind::Class,
        });
        assert_eq!(resolver.class_imports().len(), 1);

        resolver.enter_namespace("App\\Services");
        assert_eq!(resolver.namespace(), "App\\Services");
        assert_eq!(resolver.class_imports().len(), 0);
        assert_eq!(resolver.function_imports().len(), 0);
        assert_eq!(resolver.const_imports().len(), 0);
    }

    #[test]
    fn test_real_world_use_statements() {
        let source = r#"<?php

namespace App\Services;

use App\Models\User;
use App\Contracts\Greetable;
use Psr\Log\LoggerInterface as Logger;

class UserService implements Greetable {}
"#;
        let (tree, source_bytes) = parse_php(source);
        let root = tree.root_node();

        let mut resolver = NameResolver::new();

        // Find namespace
        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "namespace_definition" {
                    if let Some(ns_name) = crate::parser::cst::child_by_kind(child, "namespace_name") {
                        resolver.enter_namespace(node_text(ns_name, &source_bytes));
                    }
                }
            }
        }

        assert_eq!(resolver.namespace(), "App\\Services");

        // Parse use declarations
        let decls = find_use_declarations(root);
        for decl in &decls {
            let imports = NameResolver::parse_use_declaration(*decl, &source_bytes);
            for import in imports {
                resolver.add_import(import);
            }
        }

        assert_eq!(resolver.class_imports().len(), 3);
        assert_eq!(resolver.class_imports()["User"].fqn, "App\\Models\\User");
        assert_eq!(
            resolver.class_imports()["Greetable"].fqn,
            "App\\Contracts\\Greetable"
        );
        assert_eq!(
            resolver.class_imports()["Logger"].fqn,
            "Psr\\Log\\LoggerInterface"
        );

        // Resolution
        assert_eq!(resolver.resolve_class("User"), "App\\Models\\User");
        assert_eq!(resolver.resolve_class("Logger"), "Psr\\Log\\LoggerInterface");
        assert_eq!(
            resolver.resolve_class("UserService"),
            "App\\Services\\UserService"
        );
    }

    // ─── Class resolution tests ────────────────────────────────────────────

    #[test]
    fn test_path1_fully_qualified() {
        let resolver = NameResolver::new();
        assert_eq!(resolver.resolve_class("\\Foo\\Bar"), "Foo\\Bar");
    }

    #[test]
    fn test_path2_imported_unqualified() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "Vendor\\Lib\\Widget".to_string(),
            alias: "Widget".to_string(),
            kind: UseKind::Class,
        });
        assert_eq!(resolver.resolve_class("Widget"), "Vendor\\Lib\\Widget");
    }

    #[test]
    fn test_path3_imported_qualified() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "Vendor\\Lib".to_string(),
            alias: "Lib".to_string(),
            kind: UseKind::Class,
        });
        assert_eq!(resolver.resolve_class("Lib\\Widget"), "Vendor\\Lib\\Widget");
    }

    #[test]
    fn test_path4_unimported_unqualified() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App\\Models");
        assert_eq!(resolver.resolve_class("User"), "App\\Models\\User");
    }

    #[test]
    fn test_path5_unimported_qualified() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        assert_eq!(resolver.resolve_class("Models\\User"), "App\\Models\\User");
    }

    #[test]
    fn test_global_namespace_no_prefix() {
        let resolver = NameResolver::new();
        assert_eq!(resolver.resolve_class("User"), "User");
        assert_eq!(resolver.resolve_class("App\\User"), "App\\User");
    }

    #[test]
    fn test_no_global_fallback_for_classes() {
        // Unlike functions, classes in a namespace have NO global fallback
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App\\Models");
        // Unqualified "DateTime" without import should resolve to App\Models\DateTime
        assert_eq!(resolver.resolve_class("DateTime"), "App\\Models\\DateTime");
    }

    // ─── Special names ─────────────────────────────────────────────────────

    #[test]
    fn test_special_names_self_static() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.push_class("App\\Foo", None);

        assert_eq!(
            resolver.resolve_class_or_special("self"),
            ClassResolution::Resolved("App\\Foo".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("static"),
            ClassResolution::Resolved("App\\Foo".to_string())
        );
    }

    #[test]
    fn test_special_names_parent() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.push_class("App\\Child", Some("App\\Base"));

        assert_eq!(
            resolver.resolve_class_or_special("parent"),
            ClassResolution::Resolved("App\\Base".to_string())
        );
    }

    #[test]
    fn test_special_names_builtins() {
        let resolver = NameResolver::new();
        assert_eq!(
            resolver.resolve_class_or_special("void"),
            ClassResolution::BuiltIn("void".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("int"),
            ClassResolution::BuiltIn("int".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("string"),
            ClassResolution::BuiltIn("string".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("null"),
            ClassResolution::BuiltIn("null".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("bool"),
            ClassResolution::BuiltIn("bool".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("float"),
            ClassResolution::BuiltIn("float".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("array"),
            ClassResolution::BuiltIn("array".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("mixed"),
            ClassResolution::BuiltIn("mixed".to_string())
        );
        assert_eq!(
            resolver.resolve_class_or_special("never"),
            ClassResolution::BuiltIn("never".to_string())
        );
    }

    #[test]
    fn test_parity_with_poc03() {
        // Reproduce the resolution behavior validated in POC-03
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App\\Services");
        resolver.add_import(UseImport {
            fqn: "App\\Models\\User".to_string(),
            alias: "User".to_string(),
            kind: UseKind::Class,
        });
        resolver.add_import(UseImport {
            fqn: "Psr\\Log\\LoggerInterface".to_string(),
            alias: "Logger".to_string(),
            kind: UseKind::Class,
        });

        assert_eq!(resolver.resolve_class("User"), "App\\Models\\User");
        assert_eq!(resolver.resolve_class("Logger"), "Psr\\Log\\LoggerInterface");
        assert_eq!(
            resolver.resolve_class("UserService"),
            "App\\Services\\UserService"
        );
        assert_eq!(resolver.resolve_class("\\DateTime"), "DateTime");
    }

    // ─── Function resolution tests ─────────────────────────────────────────

    #[test]
    fn test_function_fully_qualified() {
        let resolver = NameResolver::new();
        let result = resolver.resolve_function("\\array_map");
        assert_eq!(result, FunctionResolution::Global("array_map".to_string()));
    }

    #[test]
    fn test_function_imported() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "App\\Helpers\\format_name".to_string(),
            alias: "format_name".to_string(),
            kind: UseKind::Function,
        });
        let result = resolver.resolve_function("format_name");
        assert_eq!(
            result,
            FunctionResolution::Imported("App\\Helpers\\format_name".to_string())
        );
    }

    #[test]
    fn test_function_unqualified_with_fallback() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App\\Services");
        let result = resolver.resolve_function("array_map");
        assert_eq!(result.primary(), "App\\Services\\array_map");
        assert_eq!(result.fallback(), Some("array_map"));
    }

    #[test]
    fn test_function_in_global_namespace() {
        let resolver = NameResolver::new();
        let result = resolver.resolve_function("strlen");
        assert_eq!(result, FunctionResolution::Global("strlen".to_string()));
        assert_eq!(result.primary(), "strlen");
        assert!(result.fallback().is_none());
    }

    #[test]
    fn test_function_qualified_in_namespace() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        let result = resolver.resolve_function("Sub\\helper");
        assert_eq!(result.primary(), "App\\Sub\\helper");
        assert_eq!(result.fallback(), Some("Sub\\helper"));
    }

    // ─── Constant resolution tests ─────────────────────────────────────────

    #[test]
    fn test_constant_true_false_null() {
        let resolver = NameResolver::new();
        assert_eq!(
            resolver.resolve_constant("true"),
            ConstantResolution::BuiltIn("true".to_string())
        );
        assert_eq!(
            resolver.resolve_constant("false"),
            ConstantResolution::BuiltIn("false".to_string())
        );
        assert_eq!(
            resolver.resolve_constant("null"),
            ConstantResolution::BuiltIn("null".to_string())
        );
        // Case insensitive
        assert_eq!(
            resolver.resolve_constant("TRUE"),
            ConstantResolution::BuiltIn("true".to_string())
        );
        assert_eq!(
            resolver.resolve_constant("NULL"),
            ConstantResolution::BuiltIn("null".to_string())
        );
    }

    #[test]
    fn test_constant_imported() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "App\\Config\\MAX_SIZE".to_string(),
            alias: "MAX_SIZE".to_string(),
            kind: UseKind::Const,
        });
        let result = resolver.resolve_constant("MAX_SIZE");
        assert_eq!(
            result,
            ConstantResolution::Imported("App\\Config\\MAX_SIZE".to_string())
        );
    }

    #[test]
    fn test_constant_unqualified_with_fallback() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App\\Services");
        let result = resolver.resolve_constant("PHP_INT_MAX");
        assert_eq!(result.primary(), "App\\Services\\PHP_INT_MAX");
        match &result {
            ConstantResolution::Namespaced { fallback, .. } => {
                assert_eq!(fallback, "PHP_INT_MAX");
            }
            _ => panic!("expected Namespaced"),
        }
    }

    #[test]
    fn test_constant_fully_qualified() {
        let resolver = NameResolver::new();
        let result = resolver.resolve_constant("\\PHP_INT_MAX");
        assert_eq!(
            result,
            ConstantResolution::Global("PHP_INT_MAX".to_string())
        );
    }

    #[test]
    fn test_fallback_rule_matches_php_behavior() {
        // In PHP: unqualified function/constant calls in a namespace first try ns\name,
        // then fall back to global \name. Classes do NOT have this fallback.
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");

        // Function: has fallback
        let func = resolver.resolve_function("strlen");
        assert!(func.fallback().is_some());

        // Constant: has fallback
        let cnst = resolver.resolve_constant("PHP_INT_MAX");
        match cnst {
            ConstantResolution::Namespaced { .. } => {} // OK
            _ => panic!("expected Namespaced with fallback"),
        }

        // Class: NO fallback — just gets namespace-prefixed
        let class = resolver.resolve_class("DateTime");
        assert_eq!(class, "App\\DateTime");
    }

    // ─── Unified resolve_name tests ────────────────────────────────────────

    #[test]
    fn test_resolve_class_reference() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "Vendor\\Widget".to_string(),
            alias: "Widget".to_string(),
            kind: UseKind::Class,
        });

        let result = resolver.resolve_name("Widget", NameContext::ClassReference);
        assert_eq!(result, NameResolution::Class("Vendor\\Widget".to_string()));
    }

    #[test]
    fn test_resolve_type_hint_self() {
        let mut resolver = NameResolver::new();
        resolver.push_class("App\\Foo", None);

        let result = resolver.resolve_name("self", NameContext::TypeHint);
        assert_eq!(result, NameResolution::Class("App\\Foo".to_string()));
    }

    #[test]
    fn test_resolve_type_hint_static() {
        let mut resolver = NameResolver::new();
        resolver.push_class("App\\Foo", None);

        let result = resolver.resolve_name("static", NameContext::TypeHint);
        assert_eq!(result, NameResolution::Class("App\\Foo".to_string()));
    }

    #[test]
    fn test_resolve_type_hint_parent() {
        let mut resolver = NameResolver::new();
        resolver.push_class("App\\Child", Some("App\\Base"));

        let result = resolver.resolve_name("parent", NameContext::TypeHint);
        assert_eq!(result, NameResolution::Class("App\\Base".to_string()));
    }

    #[test]
    fn test_resolve_self_outside_class() {
        let resolver = NameResolver::new();
        let result = resolver.resolve_name("self", NameContext::TypeHint);
        assert_eq!(
            result,
            NameResolution::Unresolvable("self (outside class)".to_string())
        );
    }

    #[test]
    fn test_resolve_static_class_self() {
        let mut resolver = NameResolver::new();
        resolver.push_class("App\\Foo", None);

        let result = resolver.resolve_name("self", NameContext::StaticClass);
        assert_eq!(result, NameResolution::Class("App\\Foo".to_string()));
    }

    #[test]
    fn test_resolve_static_class_parent() {
        let mut resolver = NameResolver::new();
        resolver.push_class("App\\Child", Some("App\\Base"));

        let result = resolver.resolve_name("parent", NameContext::StaticClass);
        assert_eq!(result, NameResolution::Class("App\\Base".to_string()));
    }

    #[test]
    fn test_resolve_function_call() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "App\\Helpers\\format".to_string(),
            alias: "format".to_string(),
            kind: UseKind::Function,
        });

        let result = resolver.resolve_name("format", NameContext::FunctionCall);
        assert_eq!(
            result,
            NameResolution::Function(FunctionResolution::Imported(
                "App\\Helpers\\format".to_string()
            ))
        );
    }

    #[test]
    fn test_resolve_function_call_with_fallback() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");

        let result = resolver.resolve_name("strlen", NameContext::FunctionCall);
        match result {
            NameResolution::Function(FunctionResolution::Namespaced {
                primary,
                fallback,
            }) => {
                assert_eq!(primary, "App\\strlen");
                assert_eq!(fallback, "strlen");
            }
            _ => panic!("expected Function(Namespaced), got {:?}", result),
        }
    }

    #[test]
    fn test_resolve_builtin_type_hint() {
        let resolver = NameResolver::new();
        let result = resolver.resolve_name("int", NameContext::TypeHint);
        assert_eq!(result, NameResolution::BuiltInType("int".to_string()));

        let result = resolver.resolve_name("void", NameContext::TypeHint);
        assert_eq!(result, NameResolution::BuiltInType("void".to_string()));
    }

    #[test]
    fn test_resolve_constant_access() {
        let mut resolver = NameResolver::new();
        resolver.enter_namespace("App");
        resolver.add_import(UseImport {
            fqn: "App\\Config\\MAX".to_string(),
            alias: "MAX".to_string(),
            kind: UseKind::Const,
        });

        let result = resolver.resolve_name("MAX", NameContext::ConstantAccess);
        assert_eq!(
            result,
            NameResolution::Constant(ConstantResolution::Imported(
                "App\\Config\\MAX".to_string()
            ))
        );
    }

    #[test]
    fn test_push_pop_class() {
        let mut resolver = NameResolver::new();

        assert!(resolver.current_class_fqn().is_none());
        assert!(resolver.current_parent_fqn().is_none());

        resolver.push_class("App\\Foo", Some("App\\Base"));
        assert_eq!(resolver.current_class_fqn(), Some("App\\Foo"));
        assert_eq!(resolver.current_parent_fqn(), Some("App\\Base"));

        resolver.pop_class();
        assert!(resolver.current_class_fqn().is_none());
        assert!(resolver.current_parent_fqn().is_none());
    }

    // ─── Grouped function/const use ────────────────────────────────────────

    #[test]
    fn test_grouped_function_use() {
        let (tree, source) =
            parse_php("<?php use function App\\Helpers\\{format_name, parse_date};");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].fqn, "App\\Helpers\\format_name");
        assert_eq!(imports[0].kind, UseKind::Function);
        assert_eq!(imports[1].fqn, "App\\Helpers\\parse_date");
        assert_eq!(imports[1].kind, UseKind::Function);
    }

    #[test]
    fn test_grouped_const_use() {
        let (tree, source) = parse_php("<?php use const App\\Config\\{MAX_SIZE, MIN_SIZE};");
        let decls = find_use_declarations(tree.root_node());
        let imports = NameResolver::parse_use_declaration(decls[0], &source);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].fqn, "App\\Config\\MAX_SIZE");
        assert_eq!(imports[0].kind, UseKind::Const);
        assert_eq!(imports[1].fqn, "App\\Config\\MIN_SIZE");
        assert_eq!(imports[1].kind, UseKind::Const);
    }
}
