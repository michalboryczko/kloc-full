# kloc Snapshot Tests

Regression tests for the `kloc-cli context` command. Captures JSON output for 35 test cases across all symbol types and compares against saved baselines.

## Setup

Requires cached artifacts. Generate once:
```bash
./kloc-dev.sh --id=context-final
```

## Usage

```bash
# Capture a new snapshot (baseline)
./tests/snapshot.sh capture

# Verify current code against a snapshot
./tests/snapshot.sh verify snapshot-1702260011.json

# Show diffs for failing cases
./tests/snapshot.sh diff snapshot-1702260011.json

# List available snapshots
./tests/snapshot.sh list
```

## Test Cases

35 cases defined in `cases.json`:

| Category   | Count | Flags                    | Examples                                      |
|------------|-------|--------------------------|-----------------------------------------------|
| Class      | 7     | depth 1–3                | Order, OrderService, Customer, AbstractOrderProcessor |
| Interface  | 7     | depth 1–2, with/without `--impl` | OrderRepositoryInterface, EmailSenderInterface, BaseRepositoryInterface |
| Method     | 11    | depth 2–5, with/without `--impl` | createOrder, findById, process, send, getFullAddress |
| Property   | 9     | depth 2–4                | Order::$id, Customer::$contact, Address::$street |

## Files

```
tests/
├── CLAUDE.md               # Agent instructions
├── README.md               # This file
├── cases.json              # Test case definitions
├── snapshot.sh             # CLI tool (capture/verify/diff/list)
└── snapshot-*.json         # Saved snapshots
```

## Workflow

1. Before changing `kloc-cli/src/queries/context.py`:
   ```bash
   ./tests/snapshot.sh capture    # save baseline
   ```

2. Make changes

3. Verify nothing broke:
   ```bash
   ./tests/snapshot.sh verify snapshot-DDMMYYHHMM.json
   ```

4. If intentional changes, capture new baseline:
   ```bash
   ./tests/snapshot.sh capture
   ```
