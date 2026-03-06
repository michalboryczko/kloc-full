//! Integration tests comparing Rust output against PHP fixture expected outputs.
//!
//! Run with: cargo test --test integration_test

use std::path::PathBuf;
use std::process::Command;
use scip_php_rust::discovery::discover_php_files;
use scip_php_rust::names::FileNameResolver;
use scip_php_rust::parser::PhpParser;

fn rust_binary() -> PathBuf {
    // Use the built binary from the current workspace
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove "deps"
    path.push("scip-php-rust");
    path
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn run_rust_indexer(project_root: &std::path::Path) -> (PathBuf, bool) {
    let output_dir = std::env::temp_dir().join(format!(
        "scip-php-rust-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&output_dir).unwrap();

    let status = Command::new(rust_binary())
        .arg("--project-root")
        .arg(project_root)
        .arg("--output-dir")
        .arg(&output_dir)
        .status()
        .expect("Failed to run scip-php-rust binary");

    (output_dir, status.success())
}

/// Test that the Rust binary produces valid JSON for each fixture.
/// Does NOT yet compare against expected output (that comes as implementation matures).
#[test]
fn test_rust_produces_valid_json() {
    let fixtures = fixtures_dir();
    assert!(fixtures.exists(), "fixtures dir missing: {:?}", fixtures);

    // Create a minimal project dir with one fixture
    let tmpdir = std::env::temp_dir().join(format!(
        "scip-php-rust-test-simple-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmpdir);
    std::fs::create_dir_all(&tmpdir).unwrap();
    std::fs::copy(
        fixtures.join("simple_class.php"),
        tmpdir.join("simple_class.php"),
    )
    .unwrap();
    std::fs::write(
        tmpdir.join("composer.json"),
        r#"{"name": "test/fixture", "require": {}}"#,
    )
    .unwrap();

    let (output_dir, success) = run_rust_indexer(&tmpdir);
    assert!(success, "scip-php-rust exited with non-zero status");

    // Verify index.json
    let index_path = output_dir.join("index.json");
    assert!(index_path.exists(), "index.json not created");

    let index_str = std::fs::read_to_string(&index_path).unwrap();
    let index_json: serde_json::Value =
        serde_json::from_str(&index_str).expect("index.json is not valid JSON");

    // Top-level keys must exist
    assert!(
        index_json.get("metadata").is_some(),
        "missing 'metadata' key in index.json"
    );
    assert!(
        index_json.get("documents").is_some(),
        "missing 'documents' key in index.json"
    );
    assert!(
        index_json.get("external_symbols").is_some(),
        "missing 'external_symbols' key in index.json"
    );

    // Verify calls.json
    let calls_path = output_dir.join("calls.json");
    assert!(calls_path.exists(), "calls.json not created");

    let calls_str = std::fs::read_to_string(&calls_path).unwrap();
    let calls_json: serde_json::Value =
        serde_json::from_str(&calls_str).expect("calls.json is not valid JSON");

    assert!(
        calls_json.get("calls").is_some(),
        "missing 'calls' key in calls.json"
    );
    assert!(
        calls_json.get("values").is_some(),
        "missing 'values' key in calls.json"
    );

    // Cleanup
    std::fs::remove_dir_all(&tmpdir).ok();
    std::fs::remove_dir_all(&output_dir).ok();
}

/// Snapshot test: compare document count matches expected.
/// This test will fail until Task 2+ implementation is complete.
/// Marked as #[ignore] to avoid blocking CI prematurely.
#[test]
#[ignore = "not yet implemented -- enable when Task 9+ complete"]
fn test_output_matches_expected_simple_class() {
    let fixtures = fixtures_dir();

    let tmpdir = std::env::temp_dir().join(format!(
        "scip-php-rust-test-match-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmpdir);
    std::fs::create_dir_all(&tmpdir).unwrap();
    std::fs::copy(
        fixtures.join("simple_class.php"),
        tmpdir.join("simple_class.php"),
    )
    .unwrap();
    std::fs::write(
        tmpdir.join("composer.json"),
        r#"{"name": "test/fixture", "require": {}}"#,
    )
    .unwrap();

    let (output_dir, success) = run_rust_indexer(&tmpdir);
    assert!(success, "scip-php-rust exited with non-zero status");

    let index_str = std::fs::read_to_string(output_dir.join("index.json")).unwrap();
    let index_json: serde_json::Value = serde_json::from_str(&index_str).unwrap();

    // When implementation is complete, documents should contain at least 1 entry
    let documents = index_json["documents"].as_array().unwrap();
    assert!(
        !documents.is_empty(),
        "expected at least 1 document in index.json"
    );

    // Cleanup
    std::fs::remove_dir_all(&tmpdir).ok();
    std::fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn test_file_discovery_reference_project() {
    let reference_project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../kloc-reference-project-php");

    if !reference_project.exists() {
        eprintln!("Skipping: reference project not found at {:?}", reference_project);
        return;
    }

    let files = discover_php_files(&reference_project);

    // Verify we found a reasonable number of files
    assert!(
        files.project.len() > 5,
        "Expected more than 5 project PHP files, got {}",
        files.project.len()
    );

    // All project files should be .php
    for f in &files.project {
        assert_eq!(f.extension().unwrap(), "php");
    }
}

/// Test name resolution against the use_statements.php fixture.
#[test]
fn test_name_resolution_fixture() {
    let fixture_path = fixtures_dir().join("use_statements.php");
    assert!(fixture_path.exists(), "fixture missing: {:?}", fixture_path);

    let mut parser = PhpParser::new();
    let parsed = parser
        .parse_file(&fixture_path)
        .expect("Failed to parse use_statements.php");

    let mut resolver = FileNameResolver::new();
    resolver.initialize_from_file(parsed.root(), &parsed.source);

    // Check namespace
    assert_eq!(resolver.namespace(), "App\\Services");

    // Check imports exist
    let imports = resolver.resolver().class_imports();
    assert!(imports.contains_key("User"), "missing User import");
    assert_eq!(imports["User"].fqn, "App\\Models\\User");

    assert!(imports.contains_key("Logger"), "missing Logger import");
    assert_eq!(imports["Logger"].fqn, "Psr\\Log\\LoggerInterface");

    assert!(imports.contains_key("Greetable"), "missing Greetable import");
    assert_eq!(imports["Greetable"].fqn, "App\\Contracts\\Greetable");

    // Check resolution
    assert_eq!(resolver.resolve_class("User"), "App\\Models\\User");
    assert_eq!(
        resolver.resolve_class("UserService"),
        "App\\Services\\UserService"
    );
    assert_eq!(
        resolver.resolve_class("Logger"),
        "Psr\\Log\\LoggerInterface"
    );
}
