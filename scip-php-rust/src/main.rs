use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "scip-php-rust",
    about = "PHP SCIP indexer (Rust implementation)",
    version
)]
struct Args {
    /// Root directory of the PHP project to index
    #[arg(long, value_name = "PATH")]
    project_root: PathBuf,

    /// Output directory for index.json and calls.json
    #[arg(long, value_name = "PATH", default_value = ".")]
    output_dir: PathBuf,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let project_root = args
        .project_root
        .canonicalize()
        .with_context(|| format!("project-root not found: {:?}", args.project_root))?;

    let output_dir = &args.output_dir;
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("cannot create output-dir: {:?}", output_dir))?;

    if args.verbose {
        eprintln!("scip-php-rust: indexing {:?}", project_root);
    }

    // Phase 0: Discover PHP files (stub -- will be replaced in Task 2)
    let php_files = discover_php_files(&project_root);

    if args.verbose {
        eprintln!("scip-php-rust: found {} PHP files", php_files.len());
    }

    // Phase 1: Type collection (stub)
    // Phase 2: Indexing (stub)
    // Phase 3: Write output

    write_empty_output(output_dir)
        .with_context(|| format!("failed to write output to {:?}", output_dir))?;

    if args.verbose {
        eprintln!("scip-php-rust: done");
    }

    Ok(())
}

/// Stub file discovery -- returns all *.php files recursively.
/// Will be replaced by proper Composer-aware discovery in Task 2.
fn discover_php_files(root: &std::path::Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "php"))
        .map(|e| e.path().to_owned())
        .collect()
}

/// Write empty but valid JSON output files.
/// The JSON structure matches the PHP scip-php output format.
fn write_empty_output(output_dir: &std::path::Path) -> Result<()> {
    // Empty index.json -- valid SCIP document with no files
    let index_json = serde_json::json!({
        "metadata": {
            "version": "unspecified",
            "tool_info": {
                "name": "scip-php-rust",
                "version": env!("CARGO_PKG_VERSION"),
                "arguments": []
            },
            "project_root": "",
            "text_document_encoding": "UTF8"
        },
        "documents": [],
        "external_symbols": []
    });

    let index_path = output_dir.join("index.json");
    std::fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_json)?,
    )?;

    // Empty calls.json
    let calls_json = serde_json::json!({
        "calls": [],
        "values": []
    });

    let calls_path = output_dir.join("calls.json");
    std::fs::write(
        &calls_path,
        serde_json::to_string_pretty(&calls_json)?,
    )?;

    Ok(())
}
