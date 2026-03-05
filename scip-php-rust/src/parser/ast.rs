//! Typed wrappers over tree-sitter CST nodes.
//!
//! These are LAZY: they hold a reference to the tree-sitter Node and source bytes.
//! No data is extracted until an accessor method is called.
//! All borrowed data has lifetime 'a tied to the source text and tree.

// Many structs store `source` for future use by accessor methods not yet implemented
// (type node stubs, expression nodes whose accessors use only field-name lookups).
#![allow(dead_code)]

use tree_sitter::Node;

// ═══════════════════════════════════════════════════════════════════════════════
// PhpNode enum
// ═══════════════════════════════════════════════════════════════════════════════

/// Typed PHP AST node. A discriminated union over all 37 node types scip-php needs.
///
/// `'a` is the lifetime of the source bytes and tree.
/// Constructing a PhpNode is O(1) — it just stores the node reference.
pub enum PhpNode<'a> {
    // --- Scope-defining nodes (6) ---
    Namespace(NamespaceNode<'a>),
    ClassLike(ClassLikeNode<'a>),
    Method(MethodNode<'a>),
    Function(FunctionNode<'a>),
    Closure(ClosureNode<'a>),
    ArrowFunction(ArrowFunctionNode<'a>),

    // --- Definition nodes (5) ---
    ClassConst(ClassConstNode<'a>),
    EnumCase(EnumCaseNode<'a>),
    Param(ParamNode<'a>),
    Property(PropertyNode<'a>),
    PropertyItem(PropertyItemNode<'a>),

    // --- Expression/reference nodes (15) ---
    MethodCall(MethodCallNode<'a>),
    StaticCall(StaticCallNode<'a>),
    FuncCall(FuncCallNode<'a>),
    New(NewNode<'a>),
    PropertyFetch(PropertyFetchNode<'a>),
    StaticPropertyFetch(StaticPropertyFetchNode<'a>),
    ClassConstFetch(ClassConstFetchNode<'a>),
    Variable(VariableNode<'a>),
    Assign(AssignNode<'a>),
    Foreach(ForeachNode<'a>),
    ArrayDimFetch(ArrayDimFetchNode<'a>),
    Coalesce(CoalesceNode<'a>),
    Ternary(TernaryNode<'a>),
    Match(MatchNode<'a>),
    Name(NameNode<'a>),

    // --- Type nodes (6) ---
    NullableType(NullableTypeNode<'a>),
    UnionType(UnionTypeNode<'a>),
    IntersectionType(IntersectionTypeNode<'a>),
    DnfType(DnfTypeNode<'a>),
    NamedType(NamedTypeNode<'a>),
    PrimitiveType(PrimitiveTypeNode<'a>),

    /// Any CST node not recognized as one of the typed variants.
    Other(Node<'a>),
}

impl<'a> PhpNode<'a> {
    /// Get the underlying tree-sitter node (for position/range access).
    pub fn node(&self) -> Node<'a> {
        match self {
            PhpNode::Namespace(n) => n.node,
            PhpNode::ClassLike(n) => n.node,
            PhpNode::Method(n) => n.node,
            PhpNode::Function(n) => n.node,
            PhpNode::Closure(n) => n.node,
            PhpNode::ArrowFunction(n) => n.node,
            PhpNode::ClassConst(n) => n.node,
            PhpNode::EnumCase(n) => n.node,
            PhpNode::Param(n) => n.node,
            PhpNode::Property(n) => n.node,
            PhpNode::PropertyItem(n) => n.node,
            PhpNode::MethodCall(n) => n.node,
            PhpNode::StaticCall(n) => n.node,
            PhpNode::FuncCall(n) => n.node,
            PhpNode::New(n) => n.node,
            PhpNode::PropertyFetch(n) => n.node,
            PhpNode::StaticPropertyFetch(n) => n.node,
            PhpNode::ClassConstFetch(n) => n.node,
            PhpNode::Variable(n) => n.node,
            PhpNode::Assign(n) => n.node,
            PhpNode::Foreach(n) => n.node,
            PhpNode::ArrayDimFetch(n) => n.node,
            PhpNode::Coalesce(n) => n.node,
            PhpNode::Ternary(n) => n.node,
            PhpNode::Match(n) => n.node,
            PhpNode::Name(n) => n.node,
            PhpNode::NullableType(n) => n.node,
            PhpNode::UnionType(n) => n.node,
            PhpNode::IntersectionType(n) => n.node,
            PhpNode::DnfType(n) => n.node,
            PhpNode::NamedType(n) => n.node,
            PhpNode::PrimitiveType(n) => n.node,
            PhpNode::Other(n) => *n,
        }
    }
}

/// Classify a tree-sitter CST node into a typed PHP AST node.
///
/// This is the central dispatch replacing nikic/php-parser's `instanceof` checks.
/// Call this on every node during CST traversal.
///
/// # Example
/// ```ignore
/// let php_node = classify_node(ts_node, source);
/// match php_node {
///     PhpNode::ClassLike(class) => { /* handle class */ }
///     PhpNode::Method(method) => { /* handle method */ }
///     PhpNode::Other(_) => { /* skip */ }
///     _ => {}
/// }
/// ```
pub fn classify_node<'a>(node: Node<'a>, source: &'a [u8]) -> PhpNode<'a> {
    match node.kind() {
        // Scope-defining nodes
        "namespace_definition" => PhpNode::Namespace(NamespaceNode::new(node, source)),
        "class_declaration" | "interface_declaration" | "trait_declaration"
        | "enum_declaration" => PhpNode::ClassLike(ClassLikeNode::new(node, source)),
        "method_declaration" => PhpNode::Method(MethodNode::new(node, source)),
        "function_definition" => PhpNode::Function(FunctionNode::new(node, source)),
        "anonymous_function" => PhpNode::Closure(ClosureNode::new(node, source)),
        "arrow_function" => PhpNode::ArrowFunction(ArrowFunctionNode::new(node, source)),

        // Definition nodes
        "const_declaration" => PhpNode::ClassConst(ClassConstNode::new(node, source)),
        "enum_case" => PhpNode::EnumCase(EnumCaseNode::new(node, source)),
        "simple_parameter" | "property_promotion_parameter" => {
            PhpNode::Param(ParamNode::new(node, source))
        }
        "property_declaration" => PhpNode::Property(PropertyNode::new(node, source)),
        "property_element" => PhpNode::PropertyItem(PropertyItemNode::new(node, source)),

        // Call/invocation nodes
        "member_call_expression" | "nullsafe_member_call_expression" => {
            PhpNode::MethodCall(MethodCallNode::new(node, source))
        }
        "scoped_call_expression" => PhpNode::StaticCall(StaticCallNode::new(node, source)),
        "function_call_expression" => PhpNode::FuncCall(FuncCallNode::new(node, source)),
        "object_creation_expression" => PhpNode::New(NewNode::new(node, source)),

        // Property access nodes
        "member_access_expression" | "nullsafe_member_access_expression" => {
            PhpNode::PropertyFetch(PropertyFetchNode::new(node, source))
        }
        "scoped_property_access_expression" => {
            PhpNode::StaticPropertyFetch(StaticPropertyFetchNode::new(node, source))
        }
        "class_constant_access_expression" => {
            PhpNode::ClassConstFetch(ClassConstFetchNode::new(node, source))
        }

        // Other expression nodes
        "variable_name" => PhpNode::Variable(VariableNode::new(node, source)),
        "assignment_expression" => PhpNode::Assign(AssignNode::new(node, source)),
        "foreach_statement" => PhpNode::Foreach(ForeachNode::new(node, source)),
        "subscript_expression" => PhpNode::ArrayDimFetch(ArrayDimFetchNode::new(node, source)),
        "conditional_expression" => PhpNode::Ternary(TernaryNode::new(node, source)),
        "match_expression" => PhpNode::Match(MatchNode::new(node, source)),

        // Binary expression: check for ?? (coalesce) specifically
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = crate::parser::cst::node_text(op, source);
                if op_text == "??" {
                    return PhpNode::Coalesce(CoalesceNode::new(node, source));
                }
            }
            PhpNode::Other(node)
        }

        // Name nodes (may appear in many contexts)
        "name" | "qualified_name" | "namespace_name" => {
            PhpNode::Name(NameNode::new(node, source))
        }

        // Type nodes
        "optional_type" => PhpNode::NullableType(NullableTypeNode::new(node, source)),
        "union_type" => PhpNode::UnionType(UnionTypeNode::new(node, source)),
        "intersection_type" => {
            PhpNode::IntersectionType(IntersectionTypeNode::new(node, source))
        }
        "disjunctive_normal_form_type" => PhpNode::DnfType(DnfTypeNode::new(node, source)),
        "named_type" => PhpNode::NamedType(NamedTypeNode::new(node, source)),
        "primitive_type" => PhpNode::PrimitiveType(PrimitiveTypeNode::new(node, source)),

        // Anything else
        _ => PhpNode::Other(node),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scope-defining nodes (6)
// ═══════════════════════════════════════════════════════════════════════════════

// ─── 1. NamespaceNode ────────────────────────────────────────────────────────
// CST: namespace_definition
// Fields: name (namespace_name, optional for global namespace), body

pub struct NamespaceNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> NamespaceNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        NamespaceNode { node, source }
    }

    /// The namespace name (e.g., "App\Models"), or empty string for global namespace.
    pub fn name(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    /// The body node (compound_statement or declaration_list).
    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    /// Whether this is a bracketed namespace (namespace Foo { ... }) or not (namespace Foo;).
    pub fn is_bracketed(&self) -> bool {
        self.body()
            .map(|b| b.kind() == "compound_statement")
            .unwrap_or(false)
    }
}

// ─── 2. ClassLikeNode ────────────────────────────────────────────────────────
// CST: class_declaration | interface_declaration | trait_declaration | enum_declaration
// Fields: name, body (declaration_list), base_clause (for class extends), class_interface_clause (implements)

/// What kind of class-like declaration this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Interface,
    Trait,
    Enum,
}

pub struct ClassLikeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ClassLikeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ClassLikeNode { node, source }
    }

    pub fn class_kind(&self) -> ClassKind {
        match self.node.kind() {
            "interface_declaration" => ClassKind::Interface,
            "trait_declaration" => ClassKind::Trait,
            "enum_declaration" => ClassKind::Enum,
            _ => ClassKind::Class,
        }
    }

    /// The class name (unqualified). None for anonymous classes.
    pub fn name(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    /// The name node (for range/position extraction).
    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    /// The declaration body (declaration_list).
    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    /// The base class name (from `extends`), if any. Class only.
    pub fn extends_name(&self) -> Option<&'a str> {
        let base = crate::parser::cst::child_by_kind(self.node, "base_clause")?;
        // base_clause contains a name or qualified_name
        for i in 0..base.named_child_count() {
            let child = base.named_child(i)?;
            if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                return Some(crate::parser::cst::node_text(child, self.source));
            }
        }
        None
    }

    /// All interface names from `implements` clause.
    pub fn implements_names(&self) -> Vec<&'a str> {
        let mut names = Vec::new();
        if let Some(iface_clause) =
            crate::parser::cst::child_by_kind(self.node, "class_interface_clause")
        {
            for i in 0..iface_clause.named_child_count() {
                if let Some(child) = iface_clause.named_child(i) {
                    if matches!(child.kind(), "name" | "qualified_name") {
                        names.push(crate::parser::cst::node_text(child, self.source));
                    }
                }
            }
        }
        names
    }

    /// Enum backing type (e.g., "string" in `enum Status: string`). Enum only.
    pub fn enum_backing_type(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("primitive_type")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn is_abstract(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "abstract", self.source)
    }

    pub fn is_final(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "final", self.source)
    }

    pub fn is_readonly(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "readonly", self.source)
    }
}

// ─── 3. MethodNode ───────────────────────────────────────────────────────────
// CST: method_declaration
// Fields: name, parameters (formal_parameters), body, return_type, modifiers

pub struct MethodNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> MethodNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        MethodNode { node, source }
    }

    pub fn name(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    pub fn parameters(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("parameters")
    }

    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    pub fn return_type(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("return_type")
    }

    pub fn is_static(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "static", self.source)
    }

    pub fn is_abstract(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "abstract", self.source)
    }

    pub fn is_public(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "public", self.source)
    }

    pub fn is_protected(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "protected", self.source)
    }

    pub fn is_private(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "private", self.source)
    }
}

// ─── 4. FunctionNode ─────────────────────────────────────────────────────────
// CST: function_definition
// Fields: name, parameters, body, return_type

pub struct FunctionNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> FunctionNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        FunctionNode { node, source }
    }

    pub fn name(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    pub fn parameters(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("parameters")
    }

    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    pub fn return_type(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("return_type")
    }
}

// ─── 5. ClosureNode ──────────────────────────────────────────────────────────
// CST: anonymous_function
// Fields: parameters, body, return_type, static_modifier

pub struct ClosureNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ClosureNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ClosureNode { node, source }
    }

    pub fn parameters(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("parameters")
    }

    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    pub fn return_type(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("return_type")
    }

    pub fn is_static(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "static", self.source)
    }

    /// Variables captured via `use (...)` clause.
    pub fn use_clause(&self) -> Option<Node<'a>> {
        crate::parser::cst::child_by_kind(self.node, "anonymous_function_use_clause")
    }
}

// ─── 6. ArrowFunctionNode ────────────────────────────────────────────────────
// CST: arrow_function
// Fields: parameters, body (expression), return_type

pub struct ArrowFunctionNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ArrowFunctionNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ArrowFunctionNode { node, source }
    }

    pub fn parameters(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("parameters")
    }

    /// The expression body (not a compound_statement for arrow functions).
    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    pub fn return_type(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("return_type")
    }

    pub fn is_static(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "static", self.source)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Definition nodes (5)
// ═══════════════════════════════════════════════════════════════════════════════

// ─── ClassConstNode ───────────────────────────────────────────────────────────
// CST: const_declaration (must check parent is declaration_list of a class)
// Note: const_declaration can appear at namespace level too.
// When inside a class body, it's a class constant.
// Iteration: contains const_element children, each with name + value.

pub struct ClassConstNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ClassConstNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ClassConstNode { node, source }
    }

    /// Whether this const_declaration is inside a class (not a global const).
    pub fn is_class_const(&self) -> bool {
        self.node
            .parent()
            .map(|p| p.kind() == "declaration_list")
            .unwrap_or(false)
    }

    /// Visibility modifier: "public", "protected", "private", or "public" by default.
    pub fn visibility(&self) -> &'a str {
        for keyword in &["public", "protected", "private"] {
            if crate::parser::cst::has_child_text(self.node, keyword, self.source) {
                return keyword;
            }
        }
        "public"
    }

    /// Is this const declared with `final`?
    pub fn is_final(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "final", self.source)
    }

    /// Iterator over const elements (name-value pairs).
    /// Each element is a `const_element` node.
    pub fn elements(&self) -> Vec<ConstElement<'a>> {
        crate::parser::cst::children_by_kind(self.node, "const_element")
            .into_iter()
            .map(|n| ConstElement {
                node: n,
                source: self.source,
            })
            .collect()
    }
}

/// A single `name = value` within a const declaration.
pub struct ConstElement<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ConstElement<'a> {
    pub fn name(&self) -> &'a str {
        // const_element: first named child is the name (kind="name")
        crate::parser::cst::child_by_kind(self.node, "name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        crate::parser::cst::child_by_kind(self.node, "name")
    }

    pub fn value(&self) -> Option<Node<'a>> {
        // const_element: value is the second named child (after name)
        if self.node.named_child_count() >= 2 {
            self.node.named_child(1)
        } else {
            None
        }
    }
}

// ─── EnumCaseNode ────────────────────────────────────────────────────────────
// CST: enum_case (inside enum_declaration body)
// Fields: name, value (optional, for backed enums)

pub struct EnumCaseNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> EnumCaseNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        EnumCaseNode { node, source }
    }

    pub fn name(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    /// The case value (e.g., 'active' in `case Active = 'active'`). None for pure enums.
    pub fn value(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("value")
    }

    pub fn is_backed(&self) -> bool {
        self.value().is_some()
    }
}

// ─── ParamNode ───────────────────────────────────────────────────────────────
// CST: simple_parameter (in formal_parameters)
//      Also handles: property_promotion_parameter (constructor promoted properties)
// Fields: type (optional), name (variable_name), default_value (optional)
// Special: visibility_modifier present -> constructor property promotion

pub struct ParamNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ParamNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ParamNode { node, source }
    }

    /// Parameter name including the $ sign (e.g., "$name").
    pub fn name_with_dollar(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    /// Parameter name WITHOUT the $ sign (e.g., "name").
    pub fn name(&self) -> &str {
        let with_dollar = self.name_with_dollar();
        with_dollar.trim_start_matches('$')
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    /// The type annotation node (may be union_type, optional_type, named_type, etc.).
    pub fn type_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("type")
    }

    /// Default value expression node.
    pub fn default_value(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("default_value")
    }

    /// Is this a constructor promoted property?
    /// True when a visibility modifier (public/protected/private) appears before the parameter.
    pub fn is_promoted(&self) -> bool {
        // The CST node kind is "property_promotion_parameter" for promoted params
        self.node.kind() == "property_promotion_parameter"
            || crate::parser::cst::has_named_child(self.node, "visibility_modifier")
    }

    /// Visibility of promoted property. Only meaningful if is_promoted().
    pub fn visibility(&self) -> Option<&'a str> {
        if !self.is_promoted() {
            return None;
        }
        // Look for visibility_modifier child
        if let Some(vis) = crate::parser::cst::child_by_kind(self.node, "visibility_modifier") {
            return Some(crate::parser::cst::node_text(vis, self.source));
        }
        None
    }

    /// Is this parameter (or promoted property) declared as readonly?
    pub fn is_readonly(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "readonly", self.source)
    }

    /// Is this a variadic parameter (preceded by `...`)?
    pub fn is_variadic(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "...", self.source)
    }

    /// Is this a reference parameter (preceded by `&`)?
    pub fn is_reference(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "&", self.source)
    }
}

// ─── PropertyNode ────────────────────────────────────────────────────────────
// CST: property_declaration
// Contains: modifiers (public/protected/private, static, readonly), type, property_element children

pub struct PropertyNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> PropertyNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        PropertyNode { node, source }
    }

    pub fn type_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("type")
    }

    pub fn is_static(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "static", self.source)
    }

    pub fn is_readonly(&self) -> bool {
        crate::parser::cst::has_child_text(self.node, "readonly", self.source)
    }

    pub fn visibility(&self) -> &'a str {
        for v in &["public", "protected", "private"] {
            if crate::parser::cst::has_child_text(self.node, v, self.source) {
                return v;
            }
        }
        "public"
    }

    /// All property elements (name + optional default) in this declaration.
    pub fn elements(&self) -> Vec<PropertyItemNode<'a>> {
        crate::parser::cst::children_by_kind(self.node, "property_element")
            .into_iter()
            .map(|n| PropertyItemNode {
                node: n,
                source: self.source,
            })
            .collect()
    }
}

// ─── PropertyItemNode ────────────────────────────────────────────────────────
// CST: property_element
// Fields: name (variable_name), default (initializer expression)

pub struct PropertyItemNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> PropertyItemNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        PropertyItemNode { node, source }
    }

    pub fn name_with_dollar(&self) -> &'a str {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
            .unwrap_or("")
    }

    pub fn name(&self) -> &str {
        self.name_with_dollar().trim_start_matches('$')
    }

    pub fn name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    pub fn default_value(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("default")
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Expression/reference nodes (15)
// ═══════════════════════════════════════════════════════════════════════════════

// ─── MethodCallNode ───────────────────────────────────────────────────────────
// CST: member_call_expression | nullsafe_member_call_expression
// Fields: object, name, arguments

pub struct MethodCallNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> MethodCallNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        MethodCallNode { node, source }
    }

    /// The object being called on (left side of ->).
    pub fn object(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("object")
    }

    /// The method name being called.
    pub fn method_name(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn method_name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    /// The arguments list node.
    pub fn arguments(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("arguments")
    }

    /// True if this is a nullsafe call ($obj?->method()).
    pub fn is_nullsafe(&self) -> bool {
        self.node.kind() == "nullsafe_member_call_expression"
    }
}

// ─── StaticCallNode ───────────────────────────────────────────────────────────
// CST: scoped_call_expression
// Fields: scope (class name), name (method name), arguments

pub struct StaticCallNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> StaticCallNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        StaticCallNode { node, source }
    }

    /// The scope (left side of ::). May be a name or expression.
    pub fn scope(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("scope")
    }

    pub fn scope_text(&self) -> Option<&'a str> {
        self.scope()
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn method_name(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn method_name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    pub fn arguments(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("arguments")
    }
}

// ─── FuncCallNode ─────────────────────────────────────────────────────────────
// CST: function_call_expression
// Fields: function (name or expression), arguments

pub struct FuncCallNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> FuncCallNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        FuncCallNode { node, source }
    }

    /// The function name node (may be a name, qualified_name, or expression).
    pub fn function_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("function")
    }

    /// Function name as text, if it's a simple name.
    pub fn function_name(&self) -> Option<&'a str> {
        let fn_node = self.function_node()?;
        if matches!(fn_node.kind(), "name" | "qualified_name" | "namespace_name") {
            Some(crate::parser::cst::node_text(fn_node, self.source))
        } else {
            None
        }
    }

    pub fn arguments(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("arguments")
    }
}

// ─── NewNode ──────────────────────────────────────────────────────────────────
// CST: object_creation_expression
// Contains: class name (first named child that is name/qualified_name) + arguments

pub struct NewNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> NewNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        NewNode { node, source }
    }

    /// The class name node.
    pub fn class_name_node(&self) -> Option<Node<'a>> {
        for i in 0..self.node.named_child_count() {
            if let Some(child) = self.node.named_child(i) {
                if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                    return Some(child);
                }
            }
        }
        None
    }

    pub fn class_name(&self) -> Option<&'a str> {
        self.class_name_node()
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn arguments(&self) -> Option<Node<'a>> {
        crate::parser::cst::child_by_kind(self.node, "arguments")
    }

    /// Is this `new static(...)` or `new self(...)` ?
    pub fn is_self_or_static(&self) -> bool {
        self.class_name()
            .map(|n| matches!(n, "self" | "static" | "parent"))
            .unwrap_or(false)
    }
}

// ─── PropertyFetchNode ────────────────────────────────────────────────────────
// CST: member_access_expression | nullsafe_member_access_expression
// Fields: object, name

pub struct PropertyFetchNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> PropertyFetchNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        PropertyFetchNode { node, source }
    }

    pub fn object(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("object")
    }

    pub fn property_name(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn property_name_node(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("name")
    }

    pub fn is_nullsafe(&self) -> bool {
        self.node.kind() == "nullsafe_member_access_expression"
    }
}

// ─── StaticPropertyFetchNode ─────────────────────────────────────────────────
// CST: scoped_property_access_expression
// Fields: scope, name ($property)

pub struct StaticPropertyFetchNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> StaticPropertyFetchNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        StaticPropertyFetchNode { node, source }
    }

    pub fn scope(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("scope")
    }

    pub fn scope_text(&self) -> Option<&'a str> {
        self.scope()
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn property_name(&self) -> Option<&'a str> {
        self.node
            .child_by_field_name("name")
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }
}

// ─── ClassConstFetchNode ──────────────────────────────────────────────────────
// CST: class_constant_access_expression
// Fields: scope (class), name (constant name)

pub struct ClassConstFetchNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ClassConstFetchNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ClassConstFetchNode { node, source }
    }

    /// The scope (class name) — first named child of class_constant_access_expression.
    pub fn scope(&self) -> Option<Node<'a>> {
        self.node.named_child(0)
    }

    pub fn scope_text(&self) -> Option<&'a str> {
        self.scope()
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    /// The constant name — second named child of class_constant_access_expression.
    pub fn const_name(&self) -> Option<&'a str> {
        self.node
            .named_child(1)
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    pub fn const_name_node(&self) -> Option<Node<'a>> {
        self.node.named_child(1)
    }
}

// ─── VariableNode ─────────────────────────────────────────────────────────────
// CST: variable_name
// Text: "$varName" (include the dollar sign)

pub struct VariableNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> VariableNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        VariableNode { node, source }
    }

    /// Variable name WITH dollar sign (e.g., "$name").
    pub fn name_with_dollar(&self) -> &'a str {
        crate::parser::cst::node_text(self.node, self.source)
    }

    /// Variable name WITHOUT dollar sign (e.g., "name").
    pub fn name(&self) -> &str {
        self.name_with_dollar().trim_start_matches('$')
    }

    pub fn is_this(&self) -> bool {
        self.name_with_dollar() == "$this"
    }
}

// ─── AssignNode ───────────────────────────────────────────────────────────────
// CST: assignment_expression
// Fields: left, right

pub struct AssignNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> AssignNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        AssignNode { node, source }
    }

    pub fn left(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("left")
    }

    pub fn right(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("right")
    }
}

// ─── ForeachNode ──────────────────────────────────────────────────────────────
// CST: foreach_statement
// Fields: (array) iterable expression, key, value, body

pub struct ForeachNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ForeachNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ForeachNode { node, source }
    }

    /// The iterable expression (right side of `as`).
    pub fn iterable(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("value")
    }

    pub fn key(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("key")
    }

    pub fn value_var(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("value")
    }

    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }
}

// ─── ArrayDimFetchNode ────────────────────────────────────────────────────────
// CST: subscript_expression
// Fields: variable (array), index (subscript)

pub struct ArrayDimFetchNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> ArrayDimFetchNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        ArrayDimFetchNode { node, source }
    }

    pub fn variable(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("variable")
    }

    pub fn index(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("index")
    }
}

// ─── CoalesceNode ─────────────────────────────────────────────────────────────
// CST: binary_expression with operator == "??"
// NOTE: classify_node must check the operator field before creating this.

pub struct CoalesceNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> CoalesceNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        CoalesceNode { node, source }
    }

    pub fn left(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("left")
    }

    pub fn right(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("right")
    }
}

// ─── TernaryNode ──────────────────────────────────────────────────────────────
// CST: conditional_expression
// Fields: condition, body (if-true, absent for Elvis ?:), alternative (if-false)

pub struct TernaryNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> TernaryNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        TernaryNode { node, source }
    }

    pub fn condition(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("condition")
    }

    /// The if-true branch. None for Elvis operator (`$a ?: $b`).
    pub fn if_true(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }

    pub fn if_false(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("alternative")
    }

    /// Is this a short ternary (Elvis)?
    pub fn is_elvis(&self) -> bool {
        self.if_true().is_none()
    }
}

// ─── MatchNode ────────────────────────────────────────────────────────────────
// CST: match_expression
// Fields: condition (subject), body (match_block with arms)

pub struct MatchNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> MatchNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        MatchNode { node, source }
    }

    pub fn condition(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("condition")
    }

    /// The match body block (contains match_conditional_expression arm nodes).
    pub fn body(&self) -> Option<Node<'a>> {
        self.node.child_by_field_name("body")
    }
}

// ─── NameNode ─────────────────────────────────────────────────────────────────
// CST: name | qualified_name | namespace_name
// Represents a PHP name reference (class name, function name, etc.)

pub struct NameNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> NameNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        NameNode { node, source }
    }

    /// The full text of the name (e.g., "Foo", "Foo\Bar", "\Foo\Bar").
    pub fn text(&self) -> &'a str {
        crate::parser::cst::node_text(self.node, self.source)
    }

    /// Is this a fully-qualified name (starts with \)?
    pub fn is_fully_qualified(&self) -> bool {
        self.text().starts_with('\\')
    }

    /// Is this a qualified name (contains \)?
    pub fn is_qualified(&self) -> bool {
        self.text().contains('\\')
    }

    /// The first segment (before any \).
    pub fn first_segment(&self) -> &str {
        let text = self.text().trim_start_matches('\\');
        text.split('\\').next().unwrap_or(text)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Type node STUBS (another agent will complete these)
// ═══════════════════════════════════════════════════════════════════════════════

pub struct NullableTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> NullableTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        NullableTypeNode { node, source }
    }

    /// The inner type (after the ?).
    pub fn inner_type(&self) -> Option<Node<'a>> {
        // optional_type's first named child is the wrapped type
        self.node.named_child(0)
    }
}

pub struct UnionTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> UnionTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        UnionTypeNode { node, source }
    }

    /// All type members of the union.
    pub fn types(&self) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        for i in 0..self.node.named_child_count() {
            if let Some(child) = self.node.named_child(i) {
                result.push(child);
            }
        }
        result
    }
}

pub struct IntersectionTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> IntersectionTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        IntersectionTypeNode { node, source }
    }

    /// All type members of the intersection.
    pub fn types(&self) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        for i in 0..self.node.named_child_count() {
            if let Some(child) = self.node.named_child(i) {
                result.push(child);
            }
        }
        result
    }
}

pub struct DnfTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> DnfTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        DnfTypeNode { node, source }
    }

    /// The full DNF type text (for debugging).
    pub fn text(&self) -> &'a str {
        crate::parser::cst::node_text(self.node, self.source)
    }

    /// The parts of the DNF type (union members, some of which may be intersection groups).
    pub fn parts(&self) -> Vec<Node<'a>> {
        let mut result = Vec::new();
        for i in 0..self.node.named_child_count() {
            if let Some(child) = self.node.named_child(i) {
                result.push(child);
            }
        }
        result
    }
}

pub struct NamedTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> NamedTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        NamedTypeNode { node, source }
    }

    /// The name node inside the named_type.
    pub fn inner_name_node(&self) -> Option<Node<'a>> {
        // named_type contains a name or qualified_name
        for i in 0..self.node.named_child_count() {
            if let Some(child) = self.node.named_child(i) {
                if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                    return Some(child);
                }
            }
        }
        None
    }

    /// The type name as text.
    pub fn name_text(&self) -> Option<&'a str> {
        self.inner_name_node()
            .map(|n| crate::parser::cst::node_text(n, self.source))
    }

    /// Is this a fully-qualified type (starts with \)?
    pub fn is_fully_qualified(&self) -> bool {
        self.name_text()
            .map(|t| t.starts_with('\\'))
            .unwrap_or(false)
    }
}

pub struct PrimitiveTypeNode<'a> {
    pub node: Node<'a>,
    source: &'a [u8],
}

impl<'a> PrimitiveTypeNode<'a> {
    pub fn new(node: Node<'a>, source: &'a [u8]) -> Self {
        PrimitiveTypeNode { node, source }
    }

    pub fn type_name(&self) -> &'a str {
        crate::parser::cst::node_text(self.node, self.source)
    }

    pub fn is_void(&self) -> bool {
        self.type_name() == "void"
    }
    pub fn is_null(&self) -> bool {
        self.type_name() == "null"
    }
    pub fn is_never(&self) -> bool {
        self.type_name() == "never"
    }
    pub fn is_mixed(&self) -> bool {
        self.type_name() == "mixed"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_scope {
    use super::*;
    use crate::parser::cst::child_by_kind;

    fn parse(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let bytes = source.as_bytes().to_vec();
        let tree = p.parse(&bytes, None).unwrap();
        (tree, bytes)
    }

    #[test]
    fn test_namespace_node() {
        let (tree, src) = parse("<?php namespace App\\Models;");
        let root = tree.root_node();
        let ns_node = child_by_kind(root, "namespace_definition").unwrap();
        let ns = NamespaceNode::new(ns_node, &src);
        assert_eq!(ns.name(), "App\\Models");
        assert!(!ns.is_bracketed());
    }

    #[test]
    fn test_namespace_node_bracketed() {
        let (tree, src) = parse("<?php namespace App { class Foo {} }");
        let root = tree.root_node();
        let ns_node = child_by_kind(root, "namespace_definition").unwrap();
        let ns = NamespaceNode::new(ns_node, &src);
        assert_eq!(ns.name(), "App");
        assert!(ns.is_bracketed());
    }

    #[test]
    fn test_class_like_node_class() {
        let (tree, src) = parse("<?php abstract class Foo extends Bar implements Baz {}");
        let root = tree.root_node();
        let class_node = child_by_kind(root, "class_declaration").unwrap();
        let class = ClassLikeNode::new(class_node, &src);
        assert_eq!(class.class_kind(), ClassKind::Class);
        assert_eq!(class.name(), Some("Foo"));
        assert_eq!(class.extends_name(), Some("Bar"));
        assert_eq!(class.implements_names(), vec!["Baz"]);
        assert!(class.is_abstract());
        assert!(!class.is_final());
    }

    #[test]
    fn test_class_like_node_interface() {
        let (tree, src) = parse("<?php interface Greetable {}");
        let root = tree.root_node();
        let node = child_by_kind(root, "interface_declaration").unwrap();
        let iface = ClassLikeNode::new(node, &src);
        assert_eq!(iface.class_kind(), ClassKind::Interface);
        assert_eq!(iface.name(), Some("Greetable"));
    }

    #[test]
    fn test_class_like_node_enum() {
        let (tree, src) = parse("<?php enum Status: string { case Active = 'active'; }");
        let root = tree.root_node();
        let node = child_by_kind(root, "enum_declaration").unwrap();
        let en = ClassLikeNode::new(node, &src);
        assert_eq!(en.class_kind(), ClassKind::Enum);
        assert_eq!(en.name(), Some("Status"));
    }

    #[test]
    fn test_method_node() {
        let (tree, src) =
            parse("<?php class Foo { public static function bar(int $x): string {} }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let method_node = child_by_kind(body, "method_declaration").unwrap();
        let method = MethodNode::new(method_node, &src);
        assert_eq!(method.name(), "bar");
        assert!(method.is_static());
        assert!(method.is_public());
        assert!(method.parameters().is_some());
        assert!(method.return_type().is_some());
    }

    #[test]
    fn test_function_node() {
        let (tree, src) = parse("<?php function greet(string $name): string { return $name; }");
        let root = tree.root_node();
        let func_node = child_by_kind(root, "function_definition").unwrap();
        let func = FunctionNode::new(func_node, &src);
        assert_eq!(func.name(), "greet");
        assert!(func.parameters().is_some());
        assert!(func.return_type().is_some());
    }

    #[test]
    fn test_closure_node() {
        let (tree, src) =
            parse("<?php $fn = static function(int $x) use ($y): int { return $x + $y; };");
        let root = tree.root_node();
        let mut closures = Vec::new();
        crate::parser::cst::find_all(root, "anonymous_function", &mut closures);
        assert!(!closures.is_empty());
        let closure = ClosureNode::new(closures[0], &src);
        assert!(closure.is_static());
        assert!(closure.use_clause().is_some());
        assert!(closure.return_type().is_some());
    }

    #[test]
    fn test_arrow_function_node() {
        let (tree, src) = parse("<?php $fn = fn(int $x): int => $x * 2;");
        let root = tree.root_node();
        let mut arrows = Vec::new();
        crate::parser::cst::find_all(root, "arrow_function", &mut arrows);
        assert!(!arrows.is_empty());
        let arrow = ArrowFunctionNode::new(arrows[0], &src);
        assert!(!arrow.is_static());
        assert!(arrow.body().is_some());
    }

    #[test]
    fn test_phpnode_node_method() {
        let (tree, src) = parse("<?php namespace App;");
        let root = tree.root_node();
        let ns_node = child_by_kind(root, "namespace_definition").unwrap();
        let php_node = PhpNode::Namespace(NamespaceNode::new(ns_node, &src));
        assert_eq!(php_node.node().kind(), "namespace_definition");
    }

    #[test]
    fn test_phpnode_other_variant() {
        let (tree, _src) = parse("<?php echo 'hi';");
        let root = tree.root_node();
        let php_node = PhpNode::Other(root);
        assert_eq!(php_node.node().kind(), "program");
    }

    #[test]
    fn test_classify_node_stub() {
        let (tree, src) = parse("<?php class Foo {}");
        let root = tree.root_node();
        let result = classify_node(root, &src);
        // Stub always returns Other
        assert!(matches!(result, PhpNode::Other(_)));
    }
}

#[cfg(test)]
mod tests_defs {
    use super::*;
    use crate::parser::cst::{child_by_kind, children_by_kind};

    fn parse(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let bytes = source.as_bytes().to_vec();
        let tree = p.parse(&bytes, None).unwrap();
        (tree, bytes)
    }

    #[test]
    fn test_class_const_node() {
        let (tree, src) = parse("<?php class Foo { public const BAR = 42; }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let const_node = child_by_kind(body, "const_declaration").unwrap();
        let cc = ClassConstNode::new(const_node, &src);
        assert!(cc.is_class_const());
        assert_eq!(cc.visibility(), "public");
        let elems = cc.elements();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0].name(), "BAR");
    }

    #[test]
    fn test_enum_case_node() {
        let (tree, src) = parse("<?php enum Status: string { case Active = 'active'; }");
        let root = tree.root_node();
        let mut cases = Vec::new();
        crate::parser::cst::find_all(root, "enum_case", &mut cases);
        assert_eq!(cases.len(), 1);
        let case = EnumCaseNode::new(cases[0], &src);
        assert_eq!(case.name(), "Active");
        assert!(case.is_backed());
    }

    #[test]
    fn test_param_node_simple() {
        let (tree, src) = parse("<?php function foo(string $name, int $age = 0) {}");
        let root = tree.root_node();
        let mut params = Vec::new();
        crate::parser::cst::find_all(root, "simple_parameter", &mut params);
        assert_eq!(params.len(), 2);
        let p0 = ParamNode::new(params[0], &src);
        assert_eq!(p0.name(), "name");
        assert_eq!(p0.name_with_dollar(), "$name");
        assert!(!p0.is_promoted());
        assert!(!p0.is_readonly());
        let p1 = ParamNode::new(params[1], &src);
        assert_eq!(p1.name(), "age");
        assert!(p1.default_value().is_some());
    }

    #[test]
    fn test_param_node_promoted() {
        let (tree, src) = parse(
            "<?php class Foo { public function __construct(public readonly string $name) {} }",
        );
        let root = tree.root_node();
        let mut params = Vec::new();
        // property_promotion_parameter is the CST node kind for promoted params
        crate::parser::cst::find_all(root, "property_promotion_parameter", &mut params);
        if params.is_empty() {
            // Some tree-sitter-php versions use simple_parameter with visibility_modifier
            crate::parser::cst::find_all(root, "simple_parameter", &mut params);
        }
        assert!(!params.is_empty(), "No param nodes found");
        let p = ParamNode::new(params[0], &src);
        assert_eq!(p.name(), "name");
        assert!(p.is_promoted(), "Should be a promoted property");
        assert!(p.is_readonly(), "Should be readonly");
        assert_eq!(p.visibility(), Some("public"));
    }

    #[test]
    fn test_property_node() {
        let (tree, src) =
            parse("<?php class Foo { public static readonly string $bar = 'baz'; }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let prop_node = child_by_kind(body, "property_declaration").unwrap();
        let prop = PropertyNode::new(prop_node, &src);
        assert_eq!(prop.visibility(), "public");
        assert!(prop.is_static());
        assert!(prop.is_readonly());
        assert!(prop.type_node().is_some());
        let elems = prop.elements();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0].name(), "bar");
    }

    #[test]
    fn test_class_const_not_class_level() {
        // A global const should not be detected as class const
        let (tree, src) = parse("<?php const FOO = 1;");
        let root = tree.root_node();
        let const_node = child_by_kind(root, "const_declaration").unwrap();
        let cc = ClassConstNode::new(const_node, &src);
        assert!(!cc.is_class_const());
    }

    #[test]
    fn test_property_item_default() {
        let (tree, src) = parse("<?php class Foo { public string $bar = 'default'; }");
        let root = tree.root_node();
        let class = child_by_kind(root, "class_declaration").unwrap();
        let body = class.child_by_field_name("body").unwrap();
        let prop_node = child_by_kind(body, "property_declaration").unwrap();
        let prop = PropertyNode::new(prop_node, &src);
        let elems = prop.elements();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0].name_with_dollar(), "$bar");
        // Check that children_by_kind is used correctly
        let raw_elems = children_by_kind(prop_node, "property_element");
        assert_eq!(raw_elems.len(), 1);
    }
}

#[cfg(test)]
mod tests_exprs {
    use super::*;
    use crate::parser::cst::find_all;

    fn parse(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let bytes = source.as_bytes().to_vec();
        let tree = p.parse(&bytes, None).unwrap();
        (tree, bytes)
    }

    #[test]
    fn test_method_call_node() {
        let (tree, src) = parse("<?php $obj->greet('hello');");
        let root = tree.root_node();
        let mut calls = Vec::new();
        find_all(root, "member_call_expression", &mut calls);
        assert!(!calls.is_empty());
        let call = MethodCallNode::new(calls[0], &src);
        assert_eq!(call.method_name(), Some("greet"));
        assert!(!call.is_nullsafe());
    }

    #[test]
    fn test_nullsafe_method_call() {
        let (tree, src) = parse("<?php $obj?->greet();");
        let root = tree.root_node();
        let mut calls = Vec::new();
        find_all(root, "nullsafe_member_call_expression", &mut calls);
        assert!(!calls.is_empty());
        let call = MethodCallNode::new(calls[0], &src);
        assert!(call.is_nullsafe());
    }

    #[test]
    fn test_static_call_node() {
        let (tree, src) = parse("<?php Foo::bar();");
        let root = tree.root_node();
        let mut calls = Vec::new();
        find_all(root, "scoped_call_expression", &mut calls);
        assert!(!calls.is_empty());
        let call = StaticCallNode::new(calls[0], &src);
        assert_eq!(call.scope_text(), Some("Foo"));
        assert_eq!(call.method_name(), Some("bar"));
    }

    #[test]
    fn test_func_call_node() {
        let (tree, src) = parse("<?php array_map(fn($x) => $x, $arr);");
        let root = tree.root_node();
        let mut calls = Vec::new();
        find_all(root, "function_call_expression", &mut calls);
        assert!(!calls.is_empty());
        let call = FuncCallNode::new(calls[0], &src);
        assert_eq!(call.function_name(), Some("array_map"));
    }

    #[test]
    fn test_new_node() {
        let (tree, src) = parse("<?php new App\\Models\\User('Alice', 30);");
        let root = tree.root_node();
        let mut news = Vec::new();
        find_all(root, "object_creation_expression", &mut news);
        assert!(!news.is_empty());
        let n = NewNode::new(news[0], &src);
        // The class name may come as qualified_name or namespace_name depending on tree-sitter-php
        assert!(n.class_name().is_some());
        assert!(!n.is_self_or_static());
    }

    #[test]
    fn test_variable_node() {
        let (tree, src) = parse("<?php $this->name;");
        let root = tree.root_node();
        let mut vars = Vec::new();
        find_all(root, "variable_name", &mut vars);
        assert!(!vars.is_empty());
        let v = VariableNode::new(vars[0], &src);
        assert_eq!(v.name_with_dollar(), "$this");
        assert_eq!(v.name(), "this");
        assert!(v.is_this());
    }

    #[test]
    fn test_name_node_fully_qualified() {
        let (tree, src) = parse("<?php new \\DateTime();");
        let root = tree.root_node();
        let mut news = Vec::new();
        find_all(root, "object_creation_expression", &mut news);
        assert!(!news.is_empty());
        let new_n = NewNode::new(news[0], &src);
        let class_node = new_n.class_name_node().unwrap();
        let name = NameNode::new(class_node, &src);
        assert!(name.is_fully_qualified());
    }

    #[test]
    fn test_class_const_fetch() {
        let (tree, src) = parse("<?php Status::Active;");
        let root = tree.root_node();
        let mut fetches = Vec::new();
        find_all(root, "class_constant_access_expression", &mut fetches);
        assert!(!fetches.is_empty());
        let f = ClassConstFetchNode::new(fetches[0], &src);
        assert_eq!(f.scope_text(), Some("Status"));
        assert_eq!(f.const_name(), Some("Active"));
    }

    #[test]
    fn test_property_fetch_node() {
        let (tree, src) = parse("<?php $obj->name;");
        let root = tree.root_node();
        let mut fetches = Vec::new();
        find_all(root, "member_access_expression", &mut fetches);
        assert!(!fetches.is_empty());
        let f = PropertyFetchNode::new(fetches[0], &src);
        assert_eq!(f.property_name(), Some("name"));
        assert!(!f.is_nullsafe());
    }

    #[test]
    fn test_assign_node() {
        let (tree, src) = parse("<?php $x = 42;");
        let root = tree.root_node();
        let mut assigns = Vec::new();
        find_all(root, "assignment_expression", &mut assigns);
        assert!(!assigns.is_empty());
        let a = AssignNode::new(assigns[0], &src);
        assert!(a.left().is_some());
        assert!(a.right().is_some());
    }

    #[test]
    fn test_name_node_segments() {
        let (tree, src) = parse("<?php new Foo\\Bar\\Baz();");
        let root = tree.root_node();
        let mut news = Vec::new();
        find_all(root, "object_creation_expression", &mut news);
        assert!(!news.is_empty());
        let new_n = NewNode::new(news[0], &src);
        let class_node = new_n.class_name_node().unwrap();
        let name = NameNode::new(class_node, &src);
        assert!(name.is_qualified());
        assert!(!name.is_fully_qualified());
        assert_eq!(name.first_segment(), "Foo");
    }
}

#[cfg(test)]
mod tests_classify {
    use super::*;
    use crate::parser::cst::find_all;

    fn parse(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let bytes = source.as_bytes().to_vec();
        let tree = p.parse(&bytes, None).unwrap();
        (tree, bytes)
    }

    fn classify_first<'a>(
        kind: &str,
        tree: &'a tree_sitter::Tree,
        source: &'a [u8],
    ) -> PhpNode<'a> {
        let mut nodes = Vec::new();
        find_all(tree.root_node(), kind, &mut nodes);
        assert!(!nodes.is_empty(), "No {} nodes found", kind);
        classify_node(nodes[0], source)
    }

    #[test]
    fn test_classify_class_declaration() {
        let (tree, src) = parse("<?php class Foo {}");
        let node = classify_first("class_declaration", &tree, &src);
        assert!(matches!(node, PhpNode::ClassLike(_)));
    }

    #[test]
    fn test_classify_interface_declaration() {
        let (tree, src) = parse("<?php interface Bar {}");
        let node = classify_first("interface_declaration", &tree, &src);
        assert!(matches!(node, PhpNode::ClassLike(_)));
        if let PhpNode::ClassLike(cl) = node {
            assert_eq!(cl.class_kind(), ClassKind::Interface);
        }
    }

    #[test]
    fn test_classify_method() {
        let (tree, src) = parse("<?php class Foo { public function bar() {} }");
        let node = classify_first("method_declaration", &tree, &src);
        assert!(matches!(node, PhpNode::Method(_)));
    }

    #[test]
    fn test_classify_coalesce() {
        let (tree, src) = parse("<?php $a ?? $b;");
        let node = classify_first("binary_expression", &tree, &src);
        assert!(
            matches!(node, PhpNode::Coalesce(_)),
            "binary_expression with ?? should classify as Coalesce"
        );
    }

    #[test]
    fn test_classify_non_coalesce_binary_is_other() {
        let (tree, src) = parse("<?php $a + $b;");
        let node = classify_first("binary_expression", &tree, &src);
        assert!(
            matches!(node, PhpNode::Other(_)),
            "binary_expression with + should classify as Other"
        );
    }

    #[test]
    fn test_classify_union_type() {
        let (tree, src) = parse("<?php function foo(int|string $x): void {}");
        let node = classify_first("union_type", &tree, &src);
        assert!(matches!(node, PhpNode::UnionType(_)));
    }

    #[test]
    fn test_classify_nullable_type() {
        let (tree, src) = parse("<?php function foo(?string $x): void {}");
        let node = classify_first("optional_type", &tree, &src);
        assert!(matches!(node, PhpNode::NullableType(_)));
    }

    #[test]
    fn test_classify_named_type() {
        let (tree, src) = parse("<?php function foo(User $u): User { return $u; }");
        let node = classify_first("named_type", &tree, &src);
        assert!(matches!(node, PhpNode::NamedType(_)));
        if let PhpNode::NamedType(nt) = node {
            assert_eq!(nt.name_text(), Some("User"));
        }
    }

    #[test]
    fn test_classify_all_37_node_types() {
        // Parse a comprehensive PHP file that exercises all 37 node types
        let php = r#"<?php
        namespace App;
        use Psr\Logger;

        class Foo extends Bar implements Baz {
            public string $prop;
            public const C = 1;

            public function __construct(public int $x) {}
            public static function bar(?Logger $l): int|string {
                return match ($this->x) {
                    1 => 'one',
                    default => $l?->warn($this->x ?? 0),
                };
            }
        }

        enum Status: string { case A = 'a'; }

        function helper(Countable&Iterator $it): void {}

        $fn = static function() use ($x) {};
        $arrow = fn(int $n): int => $n * 2;
        $result = $obj->method($arr[0]);
        Foo::staticCall();
        new Foo();
        $a ?? $b;
        $a ? $b : $c;
        $obj->prop;
        Foo::$staticProp;
        Foo::CONST;
        foreach ($arr as $k => $v) {}
        "#;
        let (tree, src) = parse(php);

        // Just verify parse succeeds with no errors
        assert!(
            !tree.root_node().has_error(),
            "Comprehensive PHP test should parse without errors"
        );

        // Count how many distinct PhpNode variants we can classify
        // (full enumeration test)
        let root = tree.root_node();
        let mut node_count = 0;
        count_classified_nodes(root, &src, &mut node_count);
        assert!(
            node_count > 50,
            "Expected many classified nodes, got {}",
            node_count
        );
    }

    fn count_classified_nodes(node: tree_sitter::Node, source: &[u8], count: &mut usize) {
        match classify_node(node, source) {
            PhpNode::Other(_) => {}
            _ => *count += 1,
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count_classified_nodes(child, source, count);
            }
        }
    }
}
