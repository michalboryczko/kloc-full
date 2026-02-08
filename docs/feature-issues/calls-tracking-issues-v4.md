# Calls Tracking — Issues (v4)

This document tracks issues identified during calls-tracking implementation/review.

**Feature context:**
- Spec: `docs/specs/calls-indexing.md`
- Plan: `docs/specs/calls-tracking-plan.md`
- Reference: `docs/reference/kloc-scip/calls-and-data-flow.md`
- Evidence: `docs/feature-issues/calls-tracking-v3-evidence.md`
- Previous issues (v3): `docs/feature-issues/calls-tracking-issues-v3.md`

**Scope:** Changes in `scip-php/` project

**Version:** 4
**Created:** 2026-01-30

**Status of v3 issues:**
- Issue 1 (Split values/calls): FIXED
- Issue 2 (Local variable tracking): FIXED

---

## Issue 1: Promoted constructor property types not resolved

### Context

PHP 8.0+ allows constructor property promotion:

```php
readonly class EstateContact
{
    public function __construct(
        public ?string $mainPhone = null,
        public ?string $email = null,
        // ...
    ) {
    }
}
```

When accessing these properties in a chain like `$msg->contact->email`, the `return_type` should reflect the property's declared type.

### Problem

Property access on promoted constructor properties returns `null` for `return_type`, even when the property has a declared type.

**Example — current behavior:**

```json
{
    "id": "src/Factory/SynxisHotelCreateRequestPayloadFactory.php:39:44",
    "kind": "access",
    "kind_type": "access",
    "caller": "...#createFromEstateMessage().",
    "callee": "...EstateContact#$email.",
    "return_type": null,  // ❌ Should be "scip-php builtin . string#" or nullable string
    "receiver_value_id": "...39:35",
    "location": {...}
}
```

The `$email` property is declared as `?string` in `EstateContact`, but `return_type` is null.

**Investigation:**

1. `EstateContact` is a project file at `src/Message/Estate/EstateContact.php`
2. Types are collected via `Types::collect()` which calls `collectDefs()`
3. For promoted constructor properties (`Param` with flags), code at `Types.php:651-660` should store the type:
   ```php
   if ($n->flags !== 0) {
       $p = new PropertyItem($n->var->name, $n->default, $n->getAttributes());
       $name = $this->namer->name($p);
       if ($name !== null) {
           $type = $this->typeParser->parse($n->type);
           $this->defs[$name] = $type;
       }
   }
   ```
4. But when `findDefType()` looks up the property, it returns null

**Root cause:**

In `Types.php:651-660`, when collecting promoted constructor property types:

```php
if ($n->flags !== 0) {
    $p = new PropertyItem($n->var->name, $n->default, $n->getAttributes());
    $name = $this->namer->name($p);  // ← Returns NULL!
    if ($name !== null) {
        $type = $this->typeParser->parse($n->type);
        $this->defs[$name] = $type;
    }
}
```

The `$this->namer->name($p)` returns `null` because:

1. `SymbolNamer::name(PropertyItem)` calls `classLikeName($p)` at line 456
2. `classLikeName()` walks up the parent chain via `getAttribute('parent')`
3. The synthetic `PropertyItem` has attributes copied from the `Param` node
4. `Param`'s parent is `ClassMethod` (`__construct`), not `ClassLike`
5. The parent chain is: `PropertyItem` → (no parent set) → null
6. `classLikeName()` returns `null`, so `name()` returns `null`
7. The type is never stored in `$this->defs`

### Proposed Solution

**Option A: Set parent attribute correctly**

```php
if ($n->flags !== 0) {
    $p = new PropertyItem($n->var->name, $n->default, $n->getAttributes());

    // Walk up to find the ClassLike
    $parent = $n->getAttribute('parent');  // ClassMethod __construct
    if ($parent !== null) {
        $classLike = $parent->getAttribute('parent');  // ClassLike
        if ($classLike instanceof ClassLike) {
            $p->setAttribute('parent', $classLike);
        }
    }

    $name = $this->namer->name($p);
    if ($name !== null) {
        $type = $this->typeParser->parse($n->type);
        $this->defs[$name] = $type;
    }
}
```

**Option B: Use nameProp directly (simpler)**

```php
if ($n->flags !== 0) {
    // Get class symbol from context
    $parent = $n->getAttribute('parent');  // ClassMethod
    $classLike = $parent?->getAttribute('parent');  // ClassLike
    if ($classLike instanceof ClassLike) {
        $classSymbol = $this->namer->name($classLike);
        if ($classSymbol !== null) {
            $propSymbol = $this->namer->nameProp($classSymbol, $n->var->name);
            $type = $this->typeParser->parse($n->type);
            $this->defs[$propSymbol] = $type;
        }
    }
}
```

### Expected output after fix

```json
{
    "id": "...39:44",
    "kind": "access",
    "callee": "...EstateContact#$email.",
    "return_type": "scip-php synthetic union . string|null#",
    "receiver_value_id": "...39:35"
}
```

### Edge cases

| Case | Behavior |
|------|----------|
| Promoted property with simple type (`string`) | return_type should be builtin symbol |
| Promoted property with nullable type (`?string`) | return_type should be union `string\|null#` |
| Promoted property with union type (`string\|int`) | return_type should be union symbol |
| Non-promoted property | Should already work (uses `Property` node) |

### Files to modify

- `scip-php/src/Types/Types.php` — Fix promoted constructor property type collection
- `scip-php/src/SymbolNamer.php` — Possibly add debug logging or fix context handling

---

## Issue 2: Missing intermediate result values for call chains

### Context

When processing expression chains like `$msg->contact->email`, each step produces a result that becomes the receiver for the next step. Currently, `receiver_value_id` can point to either a value ID or a call ID, mixing the two concepts.

### Problem

The current implementation does NOT create intermediate "result" values for call outputs. Instead, `receiver_value_id` points directly to the previous call ID.

**Example — current behavior:**

```php
// $hotelMessage->contact->email
```

```json
{
  "values": [
    {"id": "39:20", "kind": "parameter", "symbol": "...($hotelMessage)"}
  ],
  "calls": [
    {
      "id": "39:35",
      "kind": "access",
      "callee": "...EstateMessage#$contact.",
      "receiver_value_id": "39:20"
    },
    {
      "id": "39:44",
      "kind": "access",
      "callee": "...EstateContact#$email.",
      "receiver_value_id": "39:35"
    }
  ]
}
```

Problems:
- `receiver_value_id` on second call points to a **call ID** (39:35), not a value
- No way to attach type info to intermediate results as values
- Harder to reason about — consumers must check both arrays for every ID lookup
- Inconsistent: some receivers are values, some are calls

### Proposed Solution

**Create intermediate "result" values for every call that produces a value.**

Each call should also create a corresponding value entry with:
- Same ID as the call
- `kind: "result"`
- `type`: same as the call's `return_type`
- `source_call_id`: pointing back to the call that produced it

The `receiver_value_id` on calls will then ALWAYS point to a value (never a call).

**Updated model:**

```json
{
  "values": [
    {"id": "39:20", "kind": "parameter", "symbol": "...($hotelMessage)", "type": "EstateMessage#"},
    {"id": "39:35", "kind": "result", "type": "EstateContact|null#", "source_call_id": "39:35"},
    {"id": "39:44", "kind": "result", "type": "string#", "source_call_id": "39:44"}
  ],
  "calls": [
    {"id": "39:35", "kind": "access", "callee": "...#$contact.", "receiver_value_id": "39:20"},
    {"id": "39:44", "kind": "access", "callee": "...#$email.", "receiver_value_id": "39:35"}
  ]
}
```

Note: Value and call can share the same ID — they're in different arrays. Consumer looks up in values first.

### Expected output after fix

For chain `$a->b->c->d` passed to `function($a->b->c->d)`:

```json
{
  "values": [
    {"id": "10:5", "kind": "local", "symbol": "...local$a@10", "type": "A#"},
    {"id": "10:8", "kind": "result", "type": "B#", "source_call_id": "10:8"},
    {"id": "10:11", "kind": "result", "type": "C#", "source_call_id": "10:11"},
    {"id": "10:14", "kind": "result", "type": "D#", "source_call_id": "10:14"}
  ],
  "calls": [
    {"id": "10:8", "kind": "access", "callee": "A#$b.", "receiver_value_id": "10:5"},
    {"id": "10:11", "kind": "access", "callee": "B#$c.", "receiver_value_id": "10:8"},
    {"id": "10:14", "kind": "access", "callee": "C#$d.", "receiver_value_id": "10:11"},
    {"id": "10:0", "kind": "function", "callee": "func().", "arguments": [
      {"position": 0, "value_id": "10:14"}
    ]}
  ]
}
```

**Data flow tracing (always follows values):**

```
argument[0].value_id = "10:14"
  ↓
value "10:14" (kind: result, type: D#)
  ↓ source_call_id
call "10:14" (access ->d)
  ↓ receiver_value_id
value "10:11" (kind: result, type: C#)
  ↓ source_call_id
call "10:11" (access ->c)
  ↓ receiver_value_id
value "10:8" (kind: result, type: B#)
  ↓ source_call_id
call "10:8" (access ->b)
  ↓ receiver_value_id
value "10:5" (kind: local, $a)
```

### Implementation changes

1. **Add `result` to value kinds** in ValueRecord
2. **After creating each call**, also create a result value:
   ```php
   // In trackPropertyFetchExpression, trackMethodCall, etc.
   $callRecord = new CallRecord(...);
   $this->calls[] = $callRecord;

   // Create result value for this call
   if ($callRecord->returnType !== null) {
       $resultValue = new ValueRecord(
           id: $callRecord->id,
           kind: 'result',
           symbol: null,
           type: $callRecord->returnType,
           location: $callRecord->location,
           sourceCallId: $callRecord->id,
       );
       $this->values[] = $resultValue;
   }
   ```
3. **Update `receiver_value_id` resolution** to always use the result value ID (same as call ID, but now exists in values array)

### Edge cases

| Case | Behavior |
|------|----------|
| Call with void return | No result value created |
| Call with null return_type (unknown) | Still create result value with `type: null` for chaining |
| Chained method calls `$a->foo()->bar()` | Each method call gets a result value |
| Nested calls `foo(bar())` | `bar()` result value, referenced by foo's argument |
| Constructor `new Foo()` | Result value with type = class symbol |

### Files to modify

- `scip-php/src/Calls/ValueRecord.php` — Add `result` to valid kinds
- `scip-php/src/DocIndexer.php` — Create result values for each call
- `docs/reference/kloc-scip/calls-and-data-flow.md` — Update spec to document result values and new model

---
