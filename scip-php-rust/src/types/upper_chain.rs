//! Transitive upper chain computation.
//!
//! Builds the `transitive_uppers` map in `TypeDatabase` by performing a BFS
//! over the `uppers` (direct parent/interface/trait) relationships for every
//! class-like type in the database.

use std::collections::{HashSet, VecDeque};

use dashmap::DashMap;

use super::TypeDatabase;

/// Build transitive upper chains for all types in the database.
///
/// For each type that has direct uppers, performs a BFS to collect all
/// transitive ancestors (parents, interfaces, traits) in breadth-first order.
/// The result is stored in `db.transitive_uppers`.
///
/// Cycle detection is built-in via the visited set — cycles are simply
/// not followed, avoiding infinite loops.
pub fn build_transitive_uppers(db: &TypeDatabase) {
    // Collect all FQNs that have direct uppers
    let fqns: Vec<String> = db.uppers.iter().map(|r| r.key().clone()).collect();

    for fqn in &fqns {
        let chain = compute_upper_chain(fqn, &db.uppers);
        if !chain.is_empty() {
            db.transitive_uppers.insert(fqn.clone(), chain);
        }
    }
}

/// Compute the transitive upper chain for a single type using BFS.
///
/// Returns all ancestors in BFS order (direct parents first, then their parents, etc.).
/// Cycle detection is built-in via the visited set.
pub fn compute_upper_chain(
    start: &str,
    direct_uppers: &DashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Add start to visited set to prevent cycles back to self
    visited.insert(start.to_string());

    // Seed the queue with direct uppers
    if let Some(directs) = direct_uppers.get(start) {
        for upper in directs.value() {
            if visited.insert(upper.clone()) {
                queue.push_back(upper.clone());
            }
        }
    }

    // BFS
    while let Some(current) = queue.pop_front() {
        result.push(current.clone());

        if let Some(parents_of_current) = direct_uppers.get(&current) {
            for upper in parents_of_current.value() {
                if visited.insert(upper.clone()) {
                    queue.push_back(upper.clone());
                }
            }
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::types::{SymbolKind, TypeDef, TypeDatabase};

    fn make_def() -> TypeDef {
        TypeDef {
            kind: SymbolKind::Class,
            file_path: PathBuf::from("test.php"),
            is_abstract: false,
            is_final: false,
            is_readonly: false,
            enum_backing_type: None,
            docblock: None,
        }
    }

    #[test]
    fn test_linear_chain() {
        // A -> B -> C
        let db = TypeDatabase::new();
        db.insert_def("A", make_def());
        db.insert_def("B", make_def());
        db.insert_def("C", make_def());

        db.add_uppers("A", vec!["B".to_string()]);
        db.add_uppers("B", vec!["C".to_string()]);

        build_transitive_uppers(&db);

        let a_uppers = db.get_all_uppers("A");
        assert_eq!(a_uppers, &["B", "C"]);

        let b_uppers = db.get_all_uppers("B");
        assert_eq!(b_uppers, &["C"]);

        // C has no uppers
        assert!(db.get_all_uppers("C").is_empty());
    }

    #[test]
    fn test_diamond_pattern() {
        //     D
        //    / \
        //   B   C
        //    \ /
        //     A
        let db = TypeDatabase::new();
        db.insert_def("A", make_def());
        db.insert_def("B", make_def());
        db.insert_def("C", make_def());
        db.insert_def("D", make_def());

        db.add_uppers("A", vec!["B".to_string(), "C".to_string()]);
        db.add_uppers("B", vec!["D".to_string()]);
        db.add_uppers("C", vec!["D".to_string()]);

        build_transitive_uppers(&db);

        let a_uppers = db.get_all_uppers("A");
        // BFS order: B, C first (direct), then D (from B), D is already visited so not duplicated
        assert_eq!(a_uppers.len(), 3);
        assert_eq!(a_uppers[0], "B");
        assert_eq!(a_uppers[1], "C");
        assert_eq!(a_uppers[2], "D");
    }

    #[test]
    fn test_cycle_detection() {
        // A -> B -> C -> A (cycle!)
        let db = TypeDatabase::new();
        db.insert_def("A", make_def());
        db.insert_def("B", make_def());
        db.insert_def("C", make_def());

        db.add_uppers("A", vec!["B".to_string()]);
        db.add_uppers("B", vec!["C".to_string()]);
        db.add_uppers("C", vec!["A".to_string()]);

        // Should not panic or loop
        build_transitive_uppers(&db);

        let a_uppers = db.get_all_uppers("A");
        // B and C are visited, A is the start so it's not in the result
        assert_eq!(a_uppers.len(), 2);
        assert!(a_uppers.contains(&"B".to_string()));
        assert!(a_uppers.contains(&"C".to_string()));
    }

    #[test]
    fn test_no_uppers() {
        let db = TypeDatabase::new();
        db.insert_def("Standalone", make_def());

        build_transitive_uppers(&db);

        assert!(db.get_all_uppers("Standalone").is_empty());
    }

    #[test]
    fn test_multiple_interfaces() {
        // Class implements A, B, C (all independent)
        let db = TypeDatabase::new();
        db.insert_def("MyClass", make_def());
        db.insert_def("A", make_def());
        db.insert_def("B", make_def());
        db.insert_def("C", make_def());

        db.add_uppers(
            "MyClass",
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
        );

        build_transitive_uppers(&db);

        let uppers = db.get_all_uppers("MyClass");
        assert_eq!(uppers, &["A", "B", "C"]);
    }

    #[test]
    fn test_compute_upper_chain_standalone() {
        let direct = DashMap::new();
        direct.insert("A".to_string(), vec!["B".to_string(), "C".to_string()]);
        direct.insert("B".to_string(), vec!["D".to_string()]);

        let chain = compute_upper_chain("A", &direct);
        assert_eq!(chain, vec!["B", "C", "D"]);
    }
}
