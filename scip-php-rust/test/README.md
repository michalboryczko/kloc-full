# Snapshot Tests

## Running

```bash
# From scip-php-rust/ directory:
cargo build
./test/snapshot-test.sh ../../kloc-reference-project-php/
./test/snapshot-test.sh ../../kloc-reference-project-php/ --verbose
./test/snapshot-test.sh ../../scip-php/
```

## What it does

1. Runs PHP scip-php against the target project
2. Runs Rust scip-php-rust against the same project
3. Normalizes both JSON outputs (sort keys, strip absolute paths)
4. Diffs the two outputs and reports statistics

## Goal

As the Rust implementation matures, the diff count should decrease toward zero.
