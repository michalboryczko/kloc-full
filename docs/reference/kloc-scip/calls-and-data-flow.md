# Calls and Data Flow Tracking

## Overview

The `calls.json` file tracks all operations and values in PHP code, enabling complete data flow analysis. The format separates **values** (data holders) from **calls** (operations).

**Key principle**: Every call that produces a value also creates a corresponding **result value**. This ensures `receiver_value_id` and argument `value_id` always reference values, never calls.

Each entry has:
- A unique `id` based on source position (`file:line:col`)
- Type information
- Links to related entries for chain tracking

## File Structure

```json
{
  "version": "3.1",
  "project_root": "/absolute/path/to/project",
  "values": [
    { "id": "...", "kind": "parameter", "symbol": "...", "type": "..." },
    { "id": "...", "kind": "local", "symbol": "...", "type": "...", "source_call_id": "..." },
    { "id": "...", "kind": "result", "type": "...", "source_call_id": "..." }
  ],
  "calls": [
    { "id": "...", "kind": "method", "callee": "...", "receiver_value_id": "...", "arguments": [...] },
    { "id": "...", "kind": "access", "callee": "...", "receiver_value_id": "..." }
  ]
}
```

**Note**: A call and its result value share the same `id`. They are distinguished by which array they appear in.

## Values

Values are data holders — variables, parameters, literals, constants, and **call results**.

### Value Record Structure

```json
{
  "id": "src/Service.php:10:8",
  "kind": "local",
  "symbol": "scip-php composer . App/Service#process().local$result@10",
  "type": "scip-php composer . App/User#",
  "location": { "file": "src/Service.php", "line": 10, "col": 8 },
  "source_call_id": "src/Service.php:10:18"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier: `"{file}:{line}:{col}"` |
| `kind` | string | Value kind (see table below) |
| `symbol` | string? | SCIP symbol for the value (not for literals/results) |
| `type` | string? | Type symbol this value holds |
| `location` | object | Source position |
| `source_call_id` | string? | ID of call that produces this value |
| `source_value_id` | string? | ID of value this was assigned from (for `$a = $b`) |

### Value Kinds

| Kind | Description | Has `symbol` | Has `source_call_id` |
|------|-------------|--------------|----------------------|
| `local` | Local variable | Yes | Yes (if assigned from call) |
| `parameter` | Function/method parameter | Yes | No |
| `literal` | Literal value (string, int, array, etc.) | No | No |
| `constant` | Constant reference (regular or class constant) | Yes | No |
| `result` | Result of a call (method, property access, etc.) | No | Yes (always) |

### Result Values

Every call that produces a value also creates a **result value**:

```json
{
  "id": "src/Service.php:10:22",
  "kind": "result",
  "type": "scip-php composer . App/User#",
  "location": { "file": "src/Service.php", "line": 10, "col": 22 },
  "source_call_id": "src/Service.php:10:22"
}
```

The result value's `id` matches its source call's `id`. The `source_call_id` also points to the same call for consistency.

### Local Variable Symbols

Local variables have globally unique symbols with scope and definition line:

```
{scope}.local${name}@{line}
```

Examples:
```
scip-php composer . App/Service#process().local$result@10
scip-php composer . App/Service#process().local$user@15
scip-php composer . App/Service#process().local$user@25  // re-assignment
```

For re-assignments, each assignment gets a unique symbol (different `@line`).

## Calls

Calls are operations — method calls, property access, operators. Each call that produces a value also creates a corresponding result value in the values array.

### Call Record Structure

```json
{
  "id": "src/Service.php:10:18",
  "kind": "method",
  "kind_type": "invocation",
  "caller": "scip-php composer . App/Service#process().",
  "callee": "scip-php composer . App/Repository#find().",
  "return_type": "scip-php composer . App/User#",
  "location": { "file": "src/Service.php", "line": 10, "col": 18 },
  "receiver_value_id": "src/Service.php:10:8",
  "arguments": [...]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier: `"{file}:{line}:{col}"` |
| `kind` | string | Call kind (see table below) |
| `kind_type` | string | Category: `invocation`, `access`, or `operator` |
| `caller` | string | SCIP symbol of enclosing method/function |
| `callee` | string | SCIP symbol being called/accessed |
| `return_type` | string? | Type symbol this call produces |
| `location` | object | Source position |
| `receiver_value_id` | string? | ID of value that is the receiver (always a value, never a call) |
| `arguments` | array? | Argument bindings (for invocations) |

### Call Kinds

| Kind | `kind_type` | Description | Has `receiver_value_id` | Has `arguments` |
|------|-------------|-------------|------------------------|-----------------|
| `method` | invocation | `$obj->method()` | Yes | Yes |
| `method_static` | invocation | `Foo::method()` | No | Yes |
| `method_nullsafe` | invocation | `$obj?->method()` | Yes | Yes |
| `function` | invocation | `func()` | No | Yes |
| `constructor` | invocation | `new Foo()` | No | Yes |
| `access` | access | `$obj->property` | Yes | No |
| `access_static` | access | `Foo::$property` | No | No |
| `access_nullsafe` | access | `$obj?->property` | Yes | No |
| `access_array` | access | `$arr['key']` | Yes | No (has `key_value_id`) |
| `coalesce` | operator | `$a ?? $b` | No | No (has `left_value_id`, `right_value_id`) |
| `ternary` | operator | `$a ? $b : $c` | No | No (has operand IDs) |
| `match` | operator | `match($x) {...}` | No | No (has `subject_value_id`, `arm_value_ids`) |

## Argument Record Structure

```json
{
  "position": 0,
  "parameter": "scip-php composer . App/Repository#find().($id)",
  "value_id": "src/Service.php:10:30",
  "value_type": "scip-php builtin . int#"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `position` | int | 0-based argument index |
| `parameter` | string? | SCIP symbol of the target parameter |
| `value_id` | string | ID of the value passed as argument (always a value) |
| `value_type` | string? | Type symbol of the argument value |

**Important**: `value_id` always references a value (parameter, local, literal, constant, or result). Never a call ID.

## Chaining Example

### PHP Source

```php
class NotificationService
{
    public function notify(EstateMessage $msg, int $priority = 1): void
    {
        $profile = $msg->contact->getProfile();  // line 9
        sendEmail($profile->email, $priority);   // line 11
    }
}
```

### values Array

```json
[
  {
    "id": "src/Service.php:9:16",
    "kind": "parameter",
    "symbol": "...NotificationService#notify().($msg)",
    "type": "...EstateMessage#",
    "location": {"file": "src/Service.php", "line": 9, "col": 16}
  },
  {
    "id": "src/Service.php:9:22",
    "kind": "result",
    "type": "...EstateContact#",
    "location": {"file": "src/Service.php", "line": 9, "col": 22},
    "source_call_id": "src/Service.php:9:22"
  },
  {
    "id": "src/Service.php:9:30",
    "kind": "result",
    "type": "...Profile#",
    "location": {"file": "src/Service.php", "line": 9, "col": 30},
    "source_call_id": "src/Service.php:9:30"
  },
  {
    "id": "src/Service.php:9:8",
    "kind": "local",
    "symbol": "...#notify().local$profile@9",
    "type": "...Profile#",
    "location": {"file": "src/Service.php", "line": 9, "col": 8},
    "source_call_id": "src/Service.php:9:30"
  },
  {
    "id": "src/Service.php:11:14",
    "kind": "local",
    "symbol": "...#notify().local$profile@9",
    "type": "...Profile#",
    "location": {"file": "src/Service.php", "line": 11, "col": 14}
  },
  {
    "id": "src/Service.php:11:25",
    "kind": "result",
    "type": "...string#",
    "location": {"file": "src/Service.php", "line": 11, "col": 25},
    "source_call_id": "src/Service.php:11:25"
  },
  {
    "id": "src/Service.php:11:40",
    "kind": "parameter",
    "symbol": "...NotificationService#notify().($priority)",
    "type": "...int#",
    "location": {"file": "src/Service.php", "line": 11, "col": 40}
  }
]
```

### calls Array

```json
[
  {
    "id": "src/Service.php:9:22",
    "kind": "access",
    "kind_type": "access",
    "caller": "...NotificationService#notify().",
    "callee": "...EstateMessage#contact.",
    "return_type": "...EstateContact#",
    "location": {"file": "src/Service.php", "line": 9, "col": 22},
    "receiver_value_id": "src/Service.php:9:16"
  },
  {
    "id": "src/Service.php:9:30",
    "kind": "method",
    "kind_type": "invocation",
    "caller": "...NotificationService#notify().",
    "callee": "...EstateContact#getProfile().",
    "return_type": "...Profile#",
    "location": {"file": "src/Service.php", "line": 9, "col": 30},
    "receiver_value_id": "src/Service.php:9:22"
  },
  {
    "id": "src/Service.php:11:25",
    "kind": "access",
    "kind_type": "access",
    "caller": "...NotificationService#notify().",
    "callee": "...Profile#email.",
    "return_type": "...string#",
    "location": {"file": "src/Service.php", "line": 11, "col": 25},
    "receiver_value_id": "src/Service.php:11:14"
  },
  {
    "id": "src/Service.php:11:4",
    "kind": "function",
    "kind_type": "invocation",
    "caller": "...NotificationService#notify().",
    "callee": "...sendEmail().",
    "return_type": "...void#",
    "location": {"file": "src/Service.php", "line": 11, "col": 4},
    "arguments": [
      {"position": 0, "parameter": "...($to)", "value_id": "src/Service.php:11:25", "value_type": "...string#"},
      {"position": 1, "parameter": "...($priority)", "value_id": "src/Service.php:11:40", "value_type": "...int#"}
    ]
  }
]
```

### Data Flow Visualization

```
$msg -> contact -> getProfile()
 │        │            │
 │        │            └─► call (method) ──► result value (Profile#)
 │        │                  ↑                     │
 │        │       receiver_value_id          source_call_id
 │        │                  │                     │
 │        └─► call (access) ─┼──► result value (EstateContact#)
 │                 ↑         │           │
 │      receiver_value_id    │     source_call_id
 │                 │         │           │
 └─► value (parameter) ◄─────┘           │
                                         ↓
                               local$profile assigned
                               (source_call_id → 9:30)


sendEmail( $profile -> email,   $priority )
    │          │         │          │
    │          │         │          └─► value (parameter)
    │          │         │
    │          │         └─► call (access) ──► result value (string#)
    │          │                  ↑                    │
    │          │       receiver_value_id         source_call_id
    │          │                  │
    │          └─► value (local) ─┘
    │
    └─► call (function)
          ├─► arguments[0].value_id → result value "11:25" (string#)
          └─► arguments[1].value_id → value "11:40" (parameter)
```

## Tracing Data Flow

### From argument back to source

```python
def trace_argument(values_by_id, calls_by_id, arg):
    """Trace an argument back to its source(s).

    Always starts with a value, follows source_call_id to call,
    then receiver_value_id to the next value.
    """
    current_id = arg['value_id']
    path = []

    while current_id:
        # Always look up in values first (value_id always points to value)
        value = values_by_id.get(current_id)
        if value:
            path.append(('value', value))
            source_id = value.get('source_call_id') or value.get('source_value_id')
            if source_id:
                # Follow to the call that produced this value
                call = calls_by_id.get(source_id)
                if call:
                    path.append(('call', call))
                    current_id = call.get('receiver_value_id')
                else:
                    # source_value_id case - direct value assignment
                    current_id = source_id
            else:
                break  # No source (parameter, literal, constant)
        else:
            break

    return path
```

### Type at each step

Each value's `type` tells you the type at that point:

```
Step 1: $msg (parameter)              → EstateMessage#
Step 2: ->contact result (result)     → EstateContact#
Step 3: ->getProfile() result (result)→ Profile#
Step 4: $profile (local)              → Profile#
Step 5: ->email result (result)       → string#
```

## Operators

### Null Coalesce (`??`)

```php
$value = $foo ?? $default;
```

```json
{
  "id": "...:1:9",
  "kind": "coalesce",
  "kind_type": "operator",
  "caller": "...",
  "callee": "scip-php operator . coalesce#",
  "left_value_id": "...:1:9",
  "right_value_id": "...:1:16",
  "return_type": "scip-php union . Bar|Foo#"
}
```

Result type is union of:
- Left type with `null` removed
- Right type

### Ternary (`? :`)

```php
$value = $cond ? $true : $false;
```

```json
{
  "id": "...:1:9",
  "kind": "ternary",
  "kind_type": "operator",
  "caller": "...",
  "callee": "scip-php operator . ternary#",
  "condition_value_id": "...:1:9",
  "true_value_id": "...:1:17",
  "false_value_id": "...:1:25",
  "return_type": "scip-php union . False|True#"
}
```

### Match Expression

```php
$result = match($status) {
    'active' => new ActiveHandler(),
    default => new DefaultHandler(),
};
```

```json
{
  "id": "...:1:10",
  "kind": "match",
  "kind_type": "operator",
  "caller": "...",
  "callee": "scip-php operator . match#",
  "subject_value_id": "...:1:16",
  "arm_ids": ["...:2:17", "...:3:15"],
  "return_type": "scip-php union . ActiveHandler|DefaultHandler#"
}
```

## Array Access

```php
$config = $settings['database'];
```

```json
{
  "id": "...:1:10",
  "kind": "access_array",
  "kind_type": "access",
  "caller": "...",
  "callee": null,
  "receiver_value_id": "...:1:10",
  "key_value_id": "...:1:19",
  "return_type": "scip-php builtin . mixed#"
}
```

## Local Variable Assignments

When a call result is assigned to a local variable:

```php
$user = $this->repository->find($id);
```

The local value links to its source call:

```json
{
  "id": "...:10:8",
  "kind": "local",
  "symbol": "...#method().local$user@10",
  "type": "...User#",
  "source_call_id": "...:10:25"
}
```

For value-to-value assignments:

```php
$copy = $original;
```

```json
{
  "id": "...:15:8",
  "kind": "local",
  "symbol": "...#method().local$copy@15",
  "type": "...Foo#",
  "source_value_id": "...:15:16"
}
```

## Relationship to SCIP Index

- `caller`, `callee`, `symbol` fields use SCIP symbol format
- `type` and `return_type` may reference synthetic union types (see [union-and-intersection-types.md](./union-and-intersection-types.md))
- Parameter symbols: `ClassName#method().($paramName)`
- Local symbols: `ClassName#method().local$varName@line`
- Consumers can look up any symbol in `index.scip` for full metadata

## Consumer Guide

### Building indices

```python
# Index both arrays by ID
values_by_id = {v['id']: v for v in data['values']}
calls_by_id = {c['id']: c for c in data['calls']}

# Lookup value (for receiver_value_id, value_id)
def get_value(id):
    return values_by_id.get(id)

# Lookup call (for source_call_id)
def get_call(id):
    return calls_by_id.get(id)
```

### Finding all usages of a local variable

```python
def find_usages(values, symbol):
    return [v for v in values if v.get('symbol') == symbol]
```

### Tracing call chains

```python
def get_call_chain(values_by_id, calls_by_id, start_value_id):
    """Trace from a value back through the chain of calls."""
    chain = []
    current_id = start_value_id

    while current_id:
        value = values_by_id.get(current_id)
        if not value:
            break
        chain.append(('value', value))

        source_call_id = value.get('source_call_id')
        if not source_call_id:
            break

        call = calls_by_id.get(source_call_id)
        if not call:
            break
        chain.append(('call', call))

        current_id = call.get('receiver_value_id')

    return chain
```

### Complete Chain Example

For `$a->b->c->d` passed to `func()`:

```python
# Start from argument
arg_value_id = "10:14"  # The ->d result

chain = get_call_chain(values_by_id, calls_by_id, arg_value_id)
# Returns:
# [
#   ('value', {id: "10:14", kind: "result", type: "D#", source_call_id: "10:14"}),
#   ('call',  {id: "10:14", kind: "access", callee: "C#$d.", receiver_value_id: "10:11"}),
#   ('value', {id: "10:11", kind: "result", type: "C#", source_call_id: "10:11"}),
#   ('call',  {id: "10:11", kind: "access", callee: "B#$c.", receiver_value_id: "10:8"}),
#   ('value', {id: "10:8", kind: "result", type: "B#", source_call_id: "10:8"}),
#   ('call',  {id: "10:8", kind: "access", callee: "A#$b.", receiver_value_id: "10:5"}),
#   ('value', {id: "10:5", kind: "local", symbol: "...local$a@10", type: "A#"}),
# ]
```
