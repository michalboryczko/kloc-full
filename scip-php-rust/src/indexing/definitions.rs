//! Definition emitters for PHP declarations.
//!
//! Each emitter produces SCIP definition occurrences and SymbolInformation
//! entries for PHP declarations: classes, methods, functions, properties,
//! constants, enum cases, and parameters.

use crate::indexing::context::IndexingContext;
use crate::output::scip::Relationship;
use crate::parser::ast::*;
use crate::parser::cst::node_text;
use crate::symbol::namer::SymbolNamer;
use crate::types::phpdoc::get_docblock;

/// Convert a tree-sitter Node to a SCIP range [start_line, start_col, end_line, end_col].
fn node_to_scip_range(node: tree_sitter::Node) -> Vec<u32> {
    let start = node.start_position();
    let end = node.end_position();
    vec![start.row as u32, start.column as u32, end.row as u32, end.column as u32]
}

/// Extract enum backing type by manually scanning the CST for `:` followed by a type name.
/// This is a fallback when `enum_backing_type()` returns None (field name not in grammar).
fn extract_enum_backing_type_from_cst(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut found_colon = false;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() && node_text(child, source) == ":" {
                found_colon = true;
                continue;
            }
            if found_colon && child.is_named() {
                match child.kind() {
                    "primitive_type" | "named_type" | "name" => {
                        return Some(node_text(child, source).to_string());
                    }
                    "enum_declaration_list" | "class_interface_clause" => {
                        return None;
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Extract PHPDoc summary line (first non-tag line) for appending to documentation.
fn phpdoc_summary(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let docblock = get_docblock(node, source)?;
    // Extract the first non-empty, non-tag line from the raw docblock as summary
    let lines: Vec<&str> = docblock.lines().collect();
    for line in &lines {
        let trimmed = line.trim();
        // Skip decoration
        let s = trimmed
            .strip_prefix("/**")
            .or_else(|| trimmed.strip_prefix("*"))
            .unwrap_or(trimmed)
            .trim();
        let s = s.strip_suffix("*/").unwrap_or(s).trim();
        if s.is_empty() || s.starts_with('@') {
            continue;
        }
        return Some(s.to_string());
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10.1: Class/Interface/Trait/Enum Definitions
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit a class/interface/trait/enum definition occurrence.
pub fn emit_class_definition(node: &ClassLikeNode, ctx: &mut IndexingContext) {
    let name_node = match node.name_node() {
        Some(n) => n,
        None => return, // anonymous class, skip for now
    };

    let class_fqn = match ctx.scope.current_class() {
        Some(fqn) => fqn.to_string(),
        None => return,
    };

    let symbol = ctx.namer.symbol_for_class(&class_fqn);
    let range = node_to_scip_range(name_node);

    // Build documentation string
    let kind_str = match node.class_kind() {
        ClassKind::Class => "class",
        ClassKind::Interface => "interface",
        ClassKind::Trait => "trait",
        ClassKind::Enum => "enum",
    };

    let mut modifiers = String::new();
    if node.is_abstract() {
        modifiers.push_str("abstract ");
    }
    if node.is_final() {
        modifiers.push_str("final ");
    }
    if node.is_readonly() {
        modifiers.push_str("readonly ");
    }

    let name = node.name().unwrap_or("");

    let mut doc_line = format!("{}{} {}", modifiers, kind_str, name);

    // Enum backing type
    if node.class_kind() == ClassKind::Enum {
        if let Some(backing) = node.enum_backing_type() {
            doc_line.push_str(&format!(": {}", backing));
        } else {
            // Fallback: manually scan for `:` followed by type in the CST
            if let Some(backing) = extract_enum_backing_type_from_cst(node.node, ctx.source) {
                doc_line.push_str(&format!(": {}", backing));
            }
        }
    }

    // Extends
    if let Some(extends) = node.extends_name() {
        doc_line.push_str(&format!(" extends {}", extends));
    }

    // Implements
    let implements = node.implements_names();
    if !implements.is_empty() {
        doc_line.push_str(&format!(" implements {}", implements.join(", ")));
    }

    let mut documentation = vec![format!("```php\n{}\n```", doc_line)];

    // PHPDoc summary
    if let Some(summary) = phpdoc_summary(node.node, ctx.source) {
        documentation.push(summary);
    }

    // Build relationships
    let mut relationships = Vec::new();
    let pkg = &ctx.namer.project_package.clone();
    let ver = &ctx.namer.project_version.clone();

    if let Some(extends_name) = node.extends_name() {
        let resolved = ctx.resolver.resolve_class(extends_name);
        let extends_symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
        relationships.push(Relationship {
            symbol: extends_symbol,
            is_reference: false,
            is_implementation: false,
            is_type_definition: true,
            is_definition: false,
        });
    }

    for iface_name in &implements {
        let resolved = ctx.resolver.resolve_class(iface_name);
        let iface_symbol = ctx.namer.symbol_for_class_like(&resolved, pkg, ver);
        relationships.push(Relationship {
            symbol: iface_symbol,
            is_reference: false,
            is_implementation: true,
            is_type_definition: false,
            is_definition: false,
        });
    }

    ctx.add_definition(symbol, range, 0, documentation, relationships);

    // 10.5: Emit reference occurrences for extends/implements names
    emit_class_parent_references(node, ctx);
}

/// Emit reference occurrences for class parent types (extends, implements).
fn emit_class_parent_references(node: &ClassLikeNode, ctx: &mut IndexingContext) {
    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    // extends reference
    if let Some(extends_name) = node.extends_name() {
        // Find the extends name node in the CST for range
        if let Some(base_clause) =
            crate::parser::cst::child_by_kind(node.node, "base_clause")
        {
            for i in 0..base_clause.named_child_count() {
                if let Some(child) = base_clause.named_child(i) {
                    if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                        let resolved = ctx.resolver.resolve_class(extends_name);
                        let symbol = ctx.namer.symbol_for_class_like(&resolved, &pkg, &ver);
                        ctx.add_reference(symbol, node_to_scip_range(child));
                        break;
                    }
                }
            }
        }
    }

    // implements references
    if let Some(iface_clause) =
        crate::parser::cst::child_by_kind(node.node, "class_interface_clause")
    {
        for i in 0..iface_clause.named_child_count() {
            if let Some(child) = iface_clause.named_child(i) {
                if matches!(child.kind(), "name" | "qualified_name") {
                    let name_text = node_text(child, ctx.source);
                    let resolved = ctx.resolver.resolve_class(name_text);
                    let symbol = ctx.namer.symbol_for_class_like(&resolved, &pkg, &ver);
                    ctx.add_reference(symbol, node_to_scip_range(child));
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10.2: Method and Function Definitions
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit a method definition occurrence.
pub fn emit_method_definition(node: &MethodNode, ctx: &mut IndexingContext) {
    let name_node = match node.name_node() {
        Some(n) => n,
        None => return,
    };

    let class_fqn = match ctx.scope.current_class() {
        Some(fqn) => fqn.to_string(),
        None => return,
    };

    let method_name = node.name();
    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();
    let symbol = ctx.namer.symbol_for_method(&class_fqn, method_name, &pkg, &ver);
    let range = node_to_scip_range(name_node);

    // Build documentation
    let mut modifiers = String::new();
    if node.is_abstract() {
        modifiers.push_str("abstract ");
    }
    if node.is_public() {
        modifiers.push_str("public ");
    } else if node.is_protected() {
        modifiers.push_str("protected ");
    } else if node.is_private() {
        modifiers.push_str("private ");
    }
    if node.is_static() {
        modifiers.push_str("static ");
    }

    // Format parameters
    let params_str = format_params(node.parameters(), ctx.source);

    // Format return type
    let return_type_str = node
        .return_type()
        .map(|rt| format!(": {}", node_text(rt, ctx.source)))
        .unwrap_or_default();

    let doc_line = format!(
        "{}function {}({}){}",
        modifiers, method_name, params_str, return_type_str
    );

    let mut documentation = vec![format!("```php\n{}\n```", doc_line)];

    if let Some(summary) = phpdoc_summary(node.node, ctx.source) {
        documentation.push(summary);
    }

    ctx.add_definition(symbol, range, 0, documentation, Vec::new());

    // Emit type hint references for return type
    if let Some(rt) = node.return_type() {
        super::references::emit_type_hint_references(rt, ctx);
    }

    // Constructor promotion: emit property definitions for promoted params
    if method_name == "__construct" {
        emit_constructor_promotions(node, &class_fqn, ctx);
    }
}

/// Emit property definitions for constructor-promoted parameters.
fn emit_constructor_promotions(node: &MethodNode, class_fqn: &str, ctx: &mut IndexingContext) {
    let params_node = match node.parameters() {
        Some(n) => n,
        None => return,
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    // Iterate through parameter children
    for i in 0..params_node.named_child_count() {
        if let Some(child) = params_node.named_child(i) {
            if matches!(child.kind(), "property_promotion_parameter") {
                let param = ParamNode::new(child, ctx.source);
                if param.is_promoted() {
                    let prop_name = param.name(); // without $
                    if let Some(pname_node) = param.name_node() {
                        let prop_symbol = ctx.namer.symbol_for_property(
                            class_fqn, prop_name, &pkg, &ver,
                        );

                        // Build property documentation
                        let mut doc_parts = String::new();
                        if let Some(vis) = param.visibility() {
                            doc_parts.push_str(vis);
                            doc_parts.push(' ');
                        }
                        if param.is_readonly() {
                            doc_parts.push_str("readonly ");
                        }
                        if let Some(type_node) = param.type_node() {
                            doc_parts.push_str(node_text(type_node, ctx.source));
                            doc_parts.push(' ');
                        }
                        doc_parts.push('$');
                        doc_parts.push_str(prop_name);

                        let documentation = vec![format!("```php\n{}\n```", doc_parts)];

                        ctx.add_definition(
                            prop_symbol,
                            node_to_scip_range(pname_node),
                            0,
                            documentation,
                            Vec::new(),
                        );
                    }
                }
            }
        }
    }
}

/// Emit a function definition occurrence.
pub fn emit_function_definition(node: &FunctionNode, ctx: &mut IndexingContext) {
    let name_node = match node.name_node() {
        Some(n) => n,
        None => return,
    };

    let func_name = node.name();
    // Build FQN from resolver namespace (handles both bracketed and semicolon namespaces)
    let ns = ctx.resolver.namespace();
    let fqn = if ns.is_empty() {
        func_name.to_string()
    } else {
        format!("{}\\{}", ns, func_name)
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();
    let symbol = ctx.namer.symbol_for_function(&fqn, &pkg, &ver);
    let range = node_to_scip_range(name_node);

    // Format parameters
    let params_str = format_params(node.parameters(), ctx.source);

    // Format return type
    let return_type_str = node
        .return_type()
        .map(|rt| format!(": {}", node_text(rt, ctx.source)))
        .unwrap_or_default();

    let doc_line = format!("function {}({}){}", func_name, params_str, return_type_str);

    let mut documentation = vec![format!("```php\n{}\n```", doc_line)];

    if let Some(summary) = phpdoc_summary(node.node, ctx.source) {
        documentation.push(summary);
    }

    ctx.add_definition(symbol, range, 0, documentation, Vec::new());

    // Emit type hint references for return type
    if let Some(rt) = node.return_type() {
        super::references::emit_type_hint_references(rt, ctx);
    }
}

/// Format parameter list from a formal_parameters node.
fn format_params(params_node: Option<tree_sitter::Node>, source: &[u8]) -> String {
    let params_node = match params_node {
        Some(n) => n,
        None => return String::new(),
    };

    let mut parts = Vec::new();
    for i in 0..params_node.named_child_count() {
        if let Some(child) = params_node.named_child(i) {
            if matches!(
                child.kind(),
                "simple_parameter" | "property_promotion_parameter" | "variadic_parameter"
            ) {
                let param = ParamNode::new(child, source);
                let mut part = String::new();

                // Visibility (for promoted params)
                if let Some(vis) = param.visibility() {
                    part.push_str(vis);
                    part.push(' ');
                }

                // Readonly
                if param.is_readonly() {
                    part.push_str("readonly ");
                }

                // Type
                if let Some(type_node) = param.type_node() {
                    part.push_str(node_text(type_node, source));
                    part.push(' ');
                }

                // Variadic
                if param.is_variadic() {
                    part.push_str("...");
                }

                // Name with $
                part.push_str(param.name_with_dollar());

                // Default value
                if let Some(default) = param.default_value() {
                    part.push_str(" = ");
                    part.push_str(node_text(default, source));
                }

                parts.push(part);
            }
        }
    }

    parts.join(", ")
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10.3: Property and Constant Definitions
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit a property definition occurrence.
pub fn emit_property_definition(node: &PropertyNode, ctx: &mut IndexingContext) {
    let class_fqn = match ctx.scope.current_class() {
        Some(fqn) => fqn.to_string(),
        None => return,
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();
    let visibility = node.visibility();
    let is_static = node.is_static();
    let is_readonly = node.is_readonly();
    let type_text = node
        .type_node()
        .map(|t| node_text(t, ctx.source).to_string());

    for elem in node.elements() {
        let prop_name = elem.name(); // without $
        let name_node = match elem.name_node() {
            Some(n) => n,
            None => continue,
        };

        let symbol = ctx.namer.symbol_for_property(&class_fqn, prop_name, &pkg, &ver);
        let range = node_to_scip_range(name_node);

        // Build documentation
        let mut doc_parts = String::new();
        doc_parts.push_str(visibility);
        doc_parts.push(' ');
        if is_static {
            doc_parts.push_str("static ");
        }
        if is_readonly {
            doc_parts.push_str("readonly ");
        }
        if let Some(ref t) = type_text {
            doc_parts.push_str(t);
            doc_parts.push(' ');
        }
        doc_parts.push('$');
        doc_parts.push_str(prop_name);

        // Try to find the default value (field name or fallback to second named child)
        let default_val = elem.default_value().or_else(|| {
            if elem.node.named_child_count() >= 2 {
                elem.node.named_child(1)
            } else {
                None
            }
        });
        if let Some(default) = default_val {
            doc_parts.push_str(" = ");
            doc_parts.push_str(node_text(default, ctx.source));
        }

        let mut documentation = vec![format!("```php\n{}\n```", doc_parts)];

        if let Some(summary) = phpdoc_summary(node.node, ctx.source) {
            documentation.push(summary);
        }

        ctx.add_definition(symbol, range, 0, documentation, Vec::new());
    }

    // Emit type hint references for property type (once for the declaration)
    if let Some(type_node) = node.type_node() {
        super::references::emit_type_hint_references(type_node, ctx);
    }
}

/// Emit a class constant definition occurrence.
pub fn emit_class_const_definition(node: &ClassConstNode, ctx: &mut IndexingContext) {
    if !node.is_class_const() {
        return; // Skip global constants
    }

    let class_fqn = match ctx.scope.current_class() {
        Some(fqn) => fqn.to_string(),
        None => return,
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();
    let visibility = node.visibility();

    for elem in node.elements() {
        let const_name = elem.name();
        let name_node = match elem.name_node() {
            Some(n) => n,
            None => continue,
        };

        let symbol =
            ctx.namer
                .symbol_for_class_const(&class_fqn, const_name, &pkg, &ver);
        let range = node_to_scip_range(name_node);

        // Build documentation
        let mut doc_parts = String::new();
        doc_parts.push_str(visibility);
        doc_parts.push_str(" const ");
        doc_parts.push_str(const_name);

        if let Some(value) = elem.value() {
            doc_parts.push_str(" = ");
            doc_parts.push_str(node_text(value, ctx.source));
        }

        let mut documentation = vec![format!("```php\n{}\n```", doc_parts)];

        if let Some(summary) = phpdoc_summary(node.node, ctx.source) {
            documentation.push(summary);
        }

        ctx.add_definition(symbol, range, 0, documentation, Vec::new());
    }
}

/// Emit an enum case definition occurrence.
pub fn emit_enum_case_definition(node: &EnumCaseNode, ctx: &mut IndexingContext) {
    let enum_fqn = match ctx.scope.current_class() {
        Some(fqn) => fqn.to_string(),
        None => return,
    };

    let case_name = node.name();
    let name_node = match node.name_node() {
        Some(n) => n,
        None => return,
    };

    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();
    let symbol =
        ctx.namer
            .symbol_for_enum_case(&enum_fqn, case_name, &pkg, &ver);
    let range = node_to_scip_range(name_node);

    // Build documentation
    let mut doc_parts = format!("case {}", case_name);
    if let Some(value) = node.value() {
        doc_parts.push_str(" = ");
        doc_parts.push_str(node_text(value, ctx.source));
    }

    let documentation = vec![format!("```php\n{}\n```", doc_parts)];

    ctx.add_definition(symbol, range, 0, documentation, Vec::new());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10.4: Parameter Definitions
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit a parameter definition occurrence.
pub fn emit_param_definition(node: &ParamNode, ctx: &mut IndexingContext) {
    let name_node = match node.name_node() {
        Some(n) => n,
        None => return,
    };

    let param_name = node.name();
    let local_id = ctx.next_local_id();
    let symbol = SymbolNamer::symbol_for_param(param_name, local_id);
    let range = node_to_scip_range(name_node);

    // Build documentation
    let mut doc_parts = String::new();

    if let Some(vis) = node.visibility() {
        doc_parts.push_str(vis);
        doc_parts.push(' ');
    }
    if node.is_readonly() {
        doc_parts.push_str("readonly ");
    }
    if let Some(type_node) = node.type_node() {
        doc_parts.push_str(node_text(type_node, ctx.source));
        doc_parts.push(' ');
    }
    if node.is_variadic() {
        doc_parts.push_str("...");
    }
    doc_parts.push_str(node.name_with_dollar());

    if let Some(default) = node.default_value() {
        doc_parts.push_str(" = ");
        doc_parts.push_str(node_text(default, ctx.source));
    }

    let documentation = vec![format!("```php\n{}\n```", doc_parts)];

    ctx.add_definition(symbol, range, 0, documentation, Vec::new());

    // Emit type hint references for parameter type
    if let Some(type_node) = node.type_node() {
        super::references::emit_type_hint_references(type_node, ctx);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10.5: Trait Use References
// ═══════════════════════════════════════════════════════════════════════════════

/// Emit reference occurrences for a trait use statement (`use Trait1, Trait2;`).
pub fn emit_trait_use(node: tree_sitter::Node, ctx: &mut IndexingContext) {
    let pkg = ctx.namer.project_package.clone();
    let ver = ctx.namer.project_version.clone();

    // use_declaration contains name/qualified_name children for each trait
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            if matches!(child.kind(), "name" | "qualified_name" | "namespace_name") {
                let name_text = node_text(child, ctx.source);
                let resolved = ctx.resolver.resolve_class(name_text);
                let symbol = ctx.namer.symbol_for_class_like(&resolved, &pkg, &ver);
                ctx.add_reference(symbol, node_to_scip_range(child));
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use crate::composer::Composer;
    use crate::indexing::{index_file, FileResult, IndexingContext};
    use crate::output::scip::symbol_roles;
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

    /// Helper: find a definition occurrence by symbol substring.
    fn find_def<'a>(result: &'a FileResult, symbol_substr: &str) -> Option<&'a crate::output::scip::Occurrence> {
        result.occurrences.iter().find(|o| {
            o.symbol.contains(symbol_substr)
                && (o.symbol_roles & symbol_roles::DEFINITION) != 0
        })
    }

    /// Helper: find a symbol info by symbol substring.
    fn find_symbol_info<'a>(result: &'a FileResult, symbol_substr: &str) -> Option<&'a crate::output::scip::SymbolInformation> {
        result.symbols.iter().find(|s| s.symbol.contains(symbol_substr))
    }

    /// Helper: find all reference occurrences by symbol substring.
    fn find_refs<'a>(result: &'a FileResult, symbol_substr: &str) -> Vec<&'a crate::output::scip::Occurrence> {
        result.occurrences.iter().filter(|o| {
            o.symbol.contains(symbol_substr)
                && (o.symbol_roles & symbol_roles::REFERENCE) != 0
        }).collect()
    }

    // ── Class definitions ──────────────────────────────────────────────────

    #[test]
    fn test_class_definition_basic() {
        let source = r#"<?php
namespace App\Models;

class User {
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "User#").expect("User class def not found");
        assert!(def.symbol.contains("App/Models/User#"));

        let info = find_symbol_info(&result, "User#").expect("User symbol info not found");
        assert!(info.documentation[0].contains("class User"));
    }

    #[test]
    fn test_class_with_modifiers() {
        let source = r#"<?php
namespace App;

abstract class Base {
}
"#;
        let result = setup_and_index(source);
        let info = find_symbol_info(&result, "Base#").unwrap();
        assert!(info.documentation[0].contains("abstract class Base"));
    }

    #[test]
    fn test_class_extends_implements() {
        let source = r#"<?php
namespace App\Models;

use App\Base\Model;

class User extends Model implements Serializable {
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "User#").unwrap();
        assert!(info.documentation[0].contains("extends Model"));
        assert!(info.documentation[0].contains("implements Serializable"));

        // Relationships
        assert!(!info.relationships.is_empty());
        let extends_rel = info
            .relationships
            .iter()
            .find(|r| r.is_type_definition)
            .expect("extends relationship not found");
        assert!(extends_rel.symbol.contains("Model#"));

        let impl_rel = info
            .relationships
            .iter()
            .find(|r| r.is_implementation)
            .expect("implements relationship not found");
        assert!(impl_rel.symbol.contains("Serializable#"));
    }

    #[test]
    fn test_interface_definition() {
        let source = r#"<?php
namespace App\Contracts;

interface Cacheable {
    public function getCacheKey(): string;
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Cacheable#").unwrap();
        assert!(info.documentation[0].contains("interface Cacheable"));
    }

    #[test]
    fn test_trait_definition() {
        let source = r#"<?php
namespace App\Traits;

trait Loggable {
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Loggable#").unwrap();
        assert!(info.documentation[0].contains("trait Loggable"));
    }

    #[test]
    fn test_enum_definition() {
        let source = r#"<?php
namespace App\Enums;

enum Status: string {
    case Active = 'active';
    case Inactive = 'inactive';
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Status#").unwrap();
        assert!(info.documentation[0].contains("enum Status: string"));
    }

    #[test]
    fn test_enum_case_definition() {
        let source = r#"<?php
namespace App\Enums;

enum Color {
    case Red;
    case Green;
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "Red.").expect("Red case def not found");
        assert!(def.symbol.contains("Color#Red."));

        let info = find_symbol_info(&result, "Red.").unwrap();
        assert!(info.documentation[0].contains("case Red"));
    }

    #[test]
    fn test_backed_enum_case() {
        let source = r#"<?php
namespace App;

enum Status: string {
    case Active = 'active';
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Active.").unwrap();
        assert!(info.documentation[0].contains("case Active = 'active'"));
    }

    // ── Method definitions ─────────────────────────────────────────────────

    #[test]
    fn test_method_definition() {
        let source = r#"<?php
namespace App\Models;

class User {
    public function getName(): string {
        return 'test';
    }
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "getName().").expect("getName method def not found");
        assert!(def.symbol.contains("User#getName()."));

        let info = find_symbol_info(&result, "getName().").unwrap();
        assert!(info.documentation[0].contains("public function getName(): string"));
    }

    #[test]
    fn test_method_with_params() {
        let source = r#"<?php
namespace App;

class Foo {
    protected static function process(string $name, int $count = 0): void {
    }
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "process().").unwrap();
        let doc = &info.documentation[0];
        assert!(doc.contains("protected static function process"));
        assert!(doc.contains("string $name"));
        assert!(doc.contains("int $count = 0"));
        assert!(doc.contains(": void"));
    }

    #[test]
    fn test_constructor_promotion() {
        let source = r#"<?php
namespace App;

class Point {
    public function __construct(
        public readonly float $x,
        public readonly float $y,
    ) {}
}
"#;
        let result = setup_and_index(source);

        // Constructor method def
        let ctor_def = find_def(&result, "__construct().").expect("constructor def not found");
        assert!(ctor_def.symbol.contains("Point#__construct()."));

        // Promoted property definitions
        let x_def = find_def(&result, "$x.").expect("x property def not found");
        assert!(x_def.symbol.contains("Point#$x."));

        let y_def = find_def(&result, "$y.").expect("y property def not found");
        assert!(y_def.symbol.contains("Point#$y."));

        // x property documentation
        let x_info = find_symbol_info(&result, "$x.").unwrap();
        assert!(x_info.documentation[0].contains("public readonly float $x"));
    }

    // ── Function definitions ───────────────────────────────────────────────

    #[test]
    fn test_function_definition() {
        let source = r#"<?php
namespace App\Helpers;

function format_date(string $date): string {
    return $date;
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "format_date().").expect("format_date func def not found");
        assert!(def.symbol.contains("App/Helpers/format_date()."));

        let info = find_symbol_info(&result, "format_date().").unwrap();
        assert!(info.documentation[0].contains("function format_date(string $date): string"));
    }

    #[test]
    fn test_global_function() {
        let source = r#"<?php

function helper(): void {
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "helper().").expect("helper func def not found");
        assert!(def.symbol.contains("helper()."));
    }

    // ── Property definitions ───────────────────────────────────────────────

    #[test]
    fn test_property_definition() {
        let source = r#"<?php
namespace App;

class Config {
    private string $name = 'default';
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "$name.").expect("name property def not found");
        assert!(def.symbol.contains("Config#$name."));

        let info = find_symbol_info(&result, "$name.").unwrap();
        assert!(info.documentation[0].contains("private string $name = 'default'"));
    }

    #[test]
    fn test_static_property() {
        let source = r#"<?php
namespace App;

class Counter {
    public static int $count = 0;
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "$count.").unwrap();
        assert!(info.documentation[0].contains("public static int $count = 0"));
    }

    // ── Class constant definitions ─────────────────────────────────────────

    #[test]
    fn test_class_const_definition() {
        let source = r#"<?php
namespace App;

class Config {
    public const VERSION = '1.0.0';
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "VERSION.").expect("VERSION const def not found");
        assert!(def.symbol.contains("Config#VERSION."));

        let info = find_symbol_info(&result, "VERSION.").unwrap();
        assert!(info.documentation[0].contains("public const VERSION = '1.0.0'"));
    }

    // ── Parameter definitions ──────────────────────────────────────────────

    #[test]
    fn test_param_definition() {
        let source = r#"<?php
namespace App;

class Foo {
    public function bar(string $name): void {
    }
}
"#;
        let result = setup_and_index(source);

        // Parameter gets a local symbol
        let param_def = result.occurrences.iter().find(|o| {
            o.symbol.starts_with("local ")
                && (o.symbol_roles & symbol_roles::DEFINITION) != 0
        });
        assert!(param_def.is_some(), "param definition not found");

        // Check documentation
        let param_info = result.symbols.iter().find(|s| s.symbol.starts_with("local "));
        assert!(param_info.is_some());
        assert!(param_info.unwrap().documentation[0].contains("string $name"));
    }

    #[test]
    fn test_variadic_param() {
        let source = r#"<?php
function spread(int ...$nums): void {
}
"#;
        let result = setup_and_index(source);

        let param_info = result.symbols.iter().find(|s| s.symbol.starts_with("local "));
        assert!(param_info.is_some());
        assert!(param_info.unwrap().documentation[0].contains("int ...$nums"));
    }

    // ── Trait use references ───────────────────────────────────────────────

    #[test]
    fn test_trait_use_references() {
        let source = r#"<?php
namespace App;

class User {
    use Loggable, Cacheable;
}
"#;
        let result = setup_and_index(source);

        let loggable_refs = find_refs(&result, "Loggable#");
        assert!(
            !loggable_refs.is_empty(),
            "Loggable trait reference not found"
        );

        let cacheable_refs = find_refs(&result, "Cacheable#");
        assert!(
            !cacheable_refs.is_empty(),
            "Cacheable trait reference not found"
        );
    }

    // ── Extends/implements references ──────────────────────────────────────

    #[test]
    fn test_extends_reference() {
        let source = r#"<?php
namespace App;

class Child extends Parent {
}
"#;
        let result = setup_and_index(source);

        // There should be a reference occurrence for the extends class
        let parent_refs = find_refs(&result, "Parent#");
        assert!(
            !parent_refs.is_empty(),
            "Parent class reference not found in extends clause"
        );
    }

    #[test]
    fn test_implements_references() {
        let source = r#"<?php
namespace App;

class Service implements Runnable, Stoppable {
}
"#;
        let result = setup_and_index(source);

        let runnable_refs = find_refs(&result, "Runnable#");
        assert!(!runnable_refs.is_empty(), "Runnable reference not found");

        let stoppable_refs = find_refs(&result, "Stoppable#");
        assert!(!stoppable_refs.is_empty(), "Stoppable reference not found");
    }

    // ── PHPDoc integration ─────────────────────────────────────────────────

    #[test]
    fn test_class_with_phpdoc() {
        let source = r#"<?php
namespace App;

/** Represents a user in the system. */
class User {
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "User#").unwrap();
        assert!(info.documentation.len() >= 2);
        assert!(info.documentation[1].contains("Represents a user"));
    }

    #[test]
    fn test_method_with_phpdoc() {
        let source = r#"<?php
namespace App;

class Foo {
    /** Get the name. */
    public function getName(): string {
        return '';
    }
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "getName().").unwrap();
        assert!(info.documentation.len() >= 2);
        assert!(info.documentation[1].contains("Get the name"));
    }

    // ── Multiple declarations ──────────────────────────────────────────────

    #[test]
    fn test_multiple_class_consts() {
        let source = r#"<?php
namespace App;

class Status {
    const ACTIVE = 1;
    const INACTIVE = 0;
}
"#;
        let result = setup_and_index(source);

        assert!(find_def(&result, "ACTIVE.").is_some());
        assert!(find_def(&result, "INACTIVE.").is_some());
    }

    #[test]
    fn test_multiple_properties() {
        let source = r#"<?php
namespace App;

class Config {
    public string $name;
    protected int $age;
}
"#;
        let result = setup_and_index(source);

        assert!(find_def(&result, "$name.").is_some());
        assert!(find_def(&result, "$age.").is_some());
    }

    // ── Readonly class ─────────────────────────────────────────────────────

    #[test]
    fn test_readonly_class() {
        let source = r#"<?php
namespace App;

readonly class Value {
    public function __construct(
        public string $data,
    ) {}
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Value#").unwrap();
        assert!(info.documentation[0].contains("readonly class Value"));
    }

    // ── Final class ────────────────────────────────────────────────────────

    #[test]
    fn test_final_class() {
        let source = r#"<?php
namespace App;

final class Singleton {
}
"#;
        let result = setup_and_index(source);

        let info = find_symbol_info(&result, "Singleton#").unwrap();
        assert!(info.documentation[0].contains("final class Singleton"));
    }

    // ── Global namespace class ─────────────────────────────────────────────

    #[test]
    fn test_global_namespace_class() {
        let source = r#"<?php
class SimpleClass {
    public function method(): void {}
}
"#;
        let result = setup_and_index(source);

        let def = find_def(&result, "SimpleClass#").expect("SimpleClass def not found");
        assert!(def.symbol.contains("SimpleClass#"));
    }
}
