//! Debug dump for the TypeDatabase.
//!
//! Produces a human-readable text representation of the type database
//! sorted by FQN, useful for parity testing against the PHP scip-php implementation.

use std::io::{self, Write};

use super::{SymbolKind, TypeDatabase};

/// Dump the TypeDatabase to a writer in a human-readable format.
///
/// Output is sorted by FQN for deterministic output. Each type definition
/// is printed with its kind, file, uppers, methods, and properties.
pub fn dump_type_database<W: Write>(db: &TypeDatabase, writer: &mut W) -> io::Result<()> {
    let mut fqns: Vec<&String> = db.defs.keys().collect();
    fqns.sort();

    for fqn in fqns {
        let def = &db.defs[fqn];
        let kind_str = match &def.kind {
            SymbolKind::Class => "class",
            SymbolKind::AbstractClass => "abstract class",
            SymbolKind::FinalClass => "final class",
            SymbolKind::Interface => "interface",
            SymbolKind::Trait => "trait",
            SymbolKind::Enum => "enum",
            SymbolKind::Function => "function",
            SymbolKind::Constant => "constant",
        };

        writeln!(writer, "{} {} [{}]", kind_str, fqn, def.file_path.display())?;

        // Uppers
        if let Some(uppers) = db.uppers.get(fqn) {
            if !uppers.is_empty() {
                writeln!(writer, "  extends: {}", uppers.join(", "))?;
            }
        }

        // Enum backing type
        if let Some(ref backing) = def.enum_backing_type {
            writeln!(writer, "  backing: {}", backing)?;
        }

        // Collect and sort methods for this class
        let method_prefix = format!("{}::", fqn);
        let mut methods: Vec<&String> = db
            .method_return_types
            .keys()
            .filter(|k| k.starts_with(&method_prefix))
            .collect();
        methods.sort();

        for method_key in methods {
            let method_name = &method_key[method_prefix.len()..];
            let return_type = db
                .method_return_types
                .get(method_key)
                .and_then(|opt| opt.as_deref())
                .unwrap_or("mixed");

            let params_str = if let Some(params) = db.method_params.get(method_key) {
                params
                    .iter()
                    .map(|p| {
                        let mut s = String::new();
                        if let Some(ref th) = p.type_hint {
                            s.push_str(th);
                            s.push(' ');
                        }
                        if p.is_variadic {
                            s.push_str("...");
                        }
                        s.push('$');
                        s.push_str(&p.name);
                        if p.has_default {
                            s.push_str(" = ...");
                        }
                        s
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                String::new()
            };

            writeln!(
                writer,
                "  method {}({}): {}",
                method_name, params_str, return_type
            )?;
        }

        // Collect and sort properties for this class
        let prop_prefix = format!("{}::$", fqn);
        let mut props: Vec<&String> = db
            .property_types
            .keys()
            .filter(|k| k.starts_with(&prop_prefix))
            .collect();
        props.sort();

        for prop_key in props {
            let prop_name = &prop_key[prop_prefix.len()..];
            let type_str = db
                .property_types
                .get(prop_key)
                .and_then(|opt| opt.as_deref())
                .unwrap_or("mixed");
            writeln!(writer, "  property ${}: {}", prop_name, type_str)?;
        }
    }

    // Standalone functions
    let mut func_fqns: Vec<&String> = db.function_return_types.keys().collect();
    func_fqns.sort();

    for fqn in func_fqns {
        let return_type = db
            .function_return_types
            .get(fqn)
            .and_then(|opt| opt.as_deref())
            .unwrap_or("mixed");
        writeln!(writer, "function {}: {}", fqn, return_type)?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::types::{ParamInfo, SymbolKind, TypeDef, TypeDatabase};

    #[test]
    fn test_dump_basic() {
        let mut db = TypeDatabase::new();

        db.insert_def(
            "App\\User",
            TypeDef {
                kind: SymbolKind::Class,
                file_path: PathBuf::from("src/User.php"),
                is_abstract: false,
                is_final: false,
                is_readonly: false,
                enum_backing_type: None,
                docblock: None,
            },
        );

        db.add_uppers(
            "App\\User",
            vec!["App\\Base\\Model".to_string()],
        );

        db.add_method(
            "App\\User",
            "getName",
            Some("string".to_string()),
            vec![],
        );

        db.add_method(
            "App\\User",
            "setAge",
            Some("void".to_string()),
            vec![ParamInfo {
                name: "age".to_string(),
                type_hint: Some("int".to_string()),
                has_default: false,
                is_variadic: false,
                is_promoted: false,
                is_readonly: false,
            }],
        );

        db.add_property("App\\User", "name", Some("string".to_string()));

        let mut buf = Vec::new();
        dump_type_database(&db, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("class App\\User [src/User.php]"));
        assert!(output.contains("extends: App\\Base\\Model"));
        assert!(output.contains("method getName(): string"));
        assert!(output.contains("method setAge(int $age): void"));
        assert!(output.contains("property $name: string"));
    }

    #[test]
    fn test_dump_sorted_output() {
        let mut db = TypeDatabase::new();

        // Insert in reverse alphabetical order
        db.insert_def(
            "Z\\Last",
            TypeDef {
                kind: SymbolKind::Interface,
                file_path: PathBuf::from("z.php"),
                is_abstract: false,
                is_final: false,
                is_readonly: false,
                enum_backing_type: None,
                docblock: None,
            },
        );
        db.insert_def(
            "A\\First",
            TypeDef {
                kind: SymbolKind::Class,
                file_path: PathBuf::from("a.php"),
                is_abstract: false,
                is_final: false,
                is_readonly: false,
                enum_backing_type: None,
                docblock: None,
            },
        );

        let mut buf = Vec::new();
        dump_type_database(&db, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        let first_pos = output.find("A\\First").unwrap();
        let last_pos = output.find("Z\\Last").unwrap();
        assert!(first_pos < last_pos, "Output should be sorted by FQN");
    }

    #[test]
    fn test_dump_enum_with_backing() {
        let mut db = TypeDatabase::new();
        db.insert_def(
            "App\\Status",
            TypeDef {
                kind: SymbolKind::Enum,
                file_path: PathBuf::from("status.php"),
                is_abstract: false,
                is_final: false,
                is_readonly: false,
                enum_backing_type: Some("string".to_string()),
                docblock: None,
            },
        );

        let mut buf = Vec::new();
        dump_type_database(&db, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("enum App\\Status"));
        assert!(output.contains("backing: string"));
    }

    #[test]
    fn test_dump_function() {
        let mut db = TypeDatabase::new();
        db.function_return_types
            .insert("App\\helper".to_string(), Some("string".to_string()));

        let mut buf = Vec::new();
        dump_type_database(&db, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("function App\\helper: string"));
    }
}
