use anyhow::{Context, Result};
use clap::Parser;
use std::io::BufWriter;
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

    /// Output directory for index.json and calls.json (default: .kloc/ in project root)
    #[arg(long, value_name = "PATH")]
    output_dir: Option<PathBuf>,

    /// Number of threads for parallel indexing (default: number of logical CPUs)
    #[arg(long, value_name = "N")]
    threads: Option<usize>,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,

    /// Suppress all output except errors
    #[arg(long, short = 'q')]
    quiet: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Configure rayon thread pool
    let thread_count = args.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()
        .ok(); // Ignore error if already initialized (e.g., in tests)

    let project_root = args
        .project_root
        .canonicalize()
        .with_context(|| format!("project-root not found: {:?}", args.project_root))?;

    let output_dir = args.output_dir.unwrap_or_else(|| project_root.join(".kloc"));
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("cannot create output-dir: {:?}", output_dir))?;

    let verbose = args.verbose && !args.quiet;

    if verbose {
        eprintln!("scip-php-rust: indexing {:?} with {} threads", project_root, thread_count);
    }

    let start = std::time::Instant::now();

    // Run the full pipeline
    let (results, _composer, _namer) = scip_php_rust::pipeline::run_pipeline(&project_root, verbose)?;

    // Write output
    write_output(&results, &output_dir, &project_root)?;

    if verbose {
        let total_occurrences: usize = results.iter().map(|r| r.occurrences.len()).sum();
        let total_symbols: usize = results.iter().map(|r| r.symbols.len()).sum();
        let total_calls: usize = results.iter().map(|r| r.calls.len()).sum();
        eprintln!(
            "scip-php-rust: done in {:.2}s — {} files, {} occurrences, {} symbols, {} calls",
            start.elapsed().as_secs_f64(),
            results.len(),
            total_occurrences,
            total_symbols,
            total_calls,
        );
        eprintln!("Output: {}", output_dir.display());
    }

    Ok(())
}

/// Write index.json and calls.json output files.
fn write_output(
    results: &[scip_php_rust::indexing::FileResult],
    output_dir: &std::path::Path,
    project_root: &std::path::Path,
) -> Result<()> {
    write_index_json(results, output_dir, project_root)?;
    write_calls_json(results, output_dir)?;
    Ok(())
}

/// Write index.json with SCIP document structure.
fn write_index_json(
    results: &[scip_php_rust::indexing::FileResult],
    output_dir: &std::path::Path,
    project_root: &std::path::Path,
) -> Result<()> {
    let project_root_uri = format!("file://{}/", project_root.display());

    let documents: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "language": "PHP",
                "relative_path": r.relative_path,
                "occurrences": r.occurrences,
                "symbols": r.symbols,
            })
        })
        .collect();

    let index = serde_json::json!({
        "metadata": {
            "version": "4.0",
            "tool_info": {
                "name": "scip-php",
                "version": env!("CARGO_PKG_VERSION"),
                "arguments": []
            },
            "project_root": project_root_uri,
            "text_document_encoding": "UTF8"
        },
        "documents": documents,
        "external_symbols": []
    });

    let path = output_dir.join("index.json");
    let file = std::fs::File::create(&path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &index)
        .context("failed to serialize index.json")?;

    Ok(())
}

/// Write calls.json with call and value records.
fn write_calls_json(
    results: &[scip_php_rust::indexing::FileResult],
    output_dir: &std::path::Path,
) -> Result<()> {
    let mut all_calls: Vec<&scip_php_rust::indexing::calls::CallRecord> = results
        .iter()
        .flat_map(|r| r.calls.iter())
        .collect();
    all_calls.sort_by(|a, b| {
        a.caller.cmp(&b.caller)
            .then(a.callee.cmp(&b.callee))
            .then(a.line.cmp(&b.line))
    });

    let mut all_values: Vec<&scip_php_rust::indexing::calls::ValueRecord> = results
        .iter()
        .flat_map(|r| r.values.iter())
        .collect();
    all_values.sort_by(|a, b| {
        a.source.cmp(&b.source)
            .then(a.target.cmp(&b.target))
            .then(a.line.cmp(&b.line))
    });

    let calls_output = serde_json::json!({
        "calls": all_calls,
        "values": all_values,
    });

    let path = output_dir.join("calls.json");
    let file = std::fs::File::create(&path)
        .with_context(|| format!("cannot create {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &calls_output)
        .context("failed to serialize calls.json")?;

    Ok(())
}
