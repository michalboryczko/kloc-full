# Contract Tests Documentation

Generated: 2026-02-01 17:35:17

## Summary

| Status | Count |
|--------|-------|
| ✅ Passed | 91 |
| ❌ Failed | 0 |
| ⏭️ Skipped | 15 |
| 💥 Error | 0 |
| **Total** | **106** |


## Argument Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Argument Parameter Symbol Present | Verifies arguments have parameter symbol linking to the callee parameter definition. | `ContractTests\Tests\Argument\ArgumentBindingTest::testArgumentParameterSymbolPresent` |
| ✅ | Argument value_expr for Complex Expressions | Verifies arguments with complex expressions (like self::$nextId++) have value_expr when value_id is null. | `ContractTests\Tests\Argument\ArgumentBindingTest::testArgumentValueExprForComplexExpressions` |
| ✅ | Order Constructor Arguments | Order constructor receives correct argument types (literal, result) | `ContractTests\Tests\Argument\ArgumentBindingTest::testOrderConstructorArguments` |
| ✅ | OrderRepository Constructor in save() | Order constructor in save() receives property access results from $order | `ContractTests\Tests\Argument\ArgumentBindingTest::testOrderRepositoryConstructorArguments` |
| ✅ | checkAvailability() Receives Property Access Results | InventoryChecker receives $input property values as arguments | `ContractTests\Tests\Argument\ArgumentBindingTest::testInventoryCheckerArguments` |
| ✅ | dispatch() Receives Constructor Result | MessageBus dispatch() receives OrderCreatedMessage constructor result | `ContractTests\Tests\Argument\ArgumentBindingTest::testMessageBusDispatchArgument` |
| ✅ | findById() Receives $orderId Parameter | Argument 0 of findById() points to $orderId parameter | `ContractTests\Tests\Argument\ArgumentBindingTest::testFindByIdArgumentPointsToParameter` |
| ✅ | save() Receives $order Local | Argument 0 of save() points to $order local variable | `ContractTests\Tests\Argument\ArgumentBindingTest::testSaveArgumentPointsToOrderLocal` |
| ✅ | send() Receives customerEmail Access Result | First argument of send() points to customerEmail property access result | `ContractTests\Tests\Argument\ArgumentBindingTest::testEmailSenderReceivesCustomerEmail` |

## Callkind Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Invocation Kinds Have Arguments | Verifies all calls with kind_type=invocation have an arguments array (may be empty). | `ContractTests\Tests\CallKind\CallKindTest::testAllInvocationKindsHaveArguments` |
| ⏭️ | Array Access Kind | Verifies array access is tracked with kind=access_array. Example: self::$orders[$id]. Per schema: $arr['key'] | `ContractTests\Tests\CallKind\CallKindTest::testArrayAccessKindExists` |
| ⏭️ | Array Access on self::$orders Tracked | Verifies self::$orders[$id] array access is tracked as kind=access_array with key_value_id. | `ContractTests\Tests\CallKind\CallKindTest::testArrayAccessOnOrdersTracked` |
| ✅ | Constructor Call Kind Exists | Verifies index contains constructor calls (kind=constructor). Example: new Order(). Per schema: new Foo() | `ContractTests\Tests\CallKind\CallKindTest::testConstructorCallKindExists` |
| ✅ | Function Call Kind | Verifies function calls are tracked with kind=function. Example: sprintf(). Per schema: func() | `ContractTests\Tests\CallKind\CallKindTest::testFunctionCallKindExists` |
| ✅ | Instance Methods Have Receiver When Applicable | Verifies most instance method calls have receiver_value_id. Some calls on $this in child classes may not have it tracked. | `ContractTests\Tests\CallKind\CallKindTest::testInstanceMethodsHaveReceiver` |
| ✅ | Method Call Kind Exists | Verifies index contains instance method calls (kind=method). Example: $this->orderRepository->save(). Per schema: $obj->method() | `ContractTests\Tests\CallKind\CallKindTest::testMethodCallKindExists` |
| ⏭️ | Nullsafe Method Call Kind | Verifies nullsafe method calls are tracked with kind=method_nullsafe. Example: $obj?->method(). Per schema. | `ContractTests\Tests\CallKind\CallKindTest::testNullsafeMethodCallKind` |
| ⏭️ | Nullsafe Property Access Kind | Verifies nullsafe property access is tracked with kind=access_nullsafe. Example: $obj?->property. Per schema. | `ContractTests\Tests\CallKind\CallKindTest::testNullsafePropertyAccessKind` |
| ✅ | Order Constructor Call Tracked | Verifies new Order(...) constructor call is tracked as kind=constructor with arguments. | `ContractTests\Tests\CallKind\CallKindTest::testOrderConstructorCallTracked` |
| ✅ | OrderRepository save() Call Tracked | Verifies the $this->orderRepository->save($order) call is tracked as kind=method with correct callee. | `ContractTests\Tests\CallKind\CallKindTest::testOrderRepositorySaveCallTracked` |
| ✅ | Property Access Has Receiver When Applicable | Verifies most property access calls have receiver_value_id. $this->property in readonly classes may be handled differently. | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessHasReceiver` |
| ✅ | Property Access Kind Exists | Verifies property access is tracked with kind=access. Example: $order->customerEmail. Per schema: $obj->property | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessKindExists` |
| ✅ | Property Access on $order Tracked | Verifies $order->customerEmail property access is tracked as kind=access. | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessOnOrderTracked` |
| ⏭️ | Static Method Call Kind | Verifies static method calls are tracked with kind=method_static. Example: self::$nextId++. Per schema: Foo::method() | `ContractTests\Tests\CallKind\CallKindTest::testStaticMethodCallKindExists` |
| ⏭️ | Static Property Access Kind | Verifies static property access is tracked with kind=access_static. Example: self::$orders. Per schema: Foo::$property | `ContractTests\Tests\CallKind\CallKindTest::testStaticPropertyAccessKindExists` |
| ✅ | sprintf Function Call Tracked | Verifies sprintf() function call is tracked as kind=function. | `ContractTests\Tests\CallKind\CallKindTest::testSprintfFunctionCallTracked` |

## Chain Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Chains Terminate at Valid Source | Verifies tracing any chain backwards terminates at a parameter, local, literal, or constant (not orphaned). | `ContractTests\Tests\Chain\ChainIntegrityTest::testAllChainsTerminateAtValidSource` |
| ✅ | Argument Value IDs Point to Values | Verifies every argument value_id points to an existing value entry (never a call). Per schema: argument value_id MUST reference a value. | `ContractTests\Tests\Chain\ChainIntegrityTest::testArgumentValueIdsPointToValues` |
| ✅ | Every Call Has Corresponding Result Value | Verifies each call has a result value with the same ID in the values array. Per schema: calls and result values share the same ID. | `ContractTests\Tests\Chain\ChainIntegrityTest::testEveryCallHasResultValue` |
| ✅ | Method Chain $this->orderRepository->save() | Verifies the chain $this->orderRepository->save() is traceable: $this (value) -> orderRepository (access) -> result (value) -> save (method) -> result (value). | `ContractTests\Tests\Chain\ChainIntegrityTest::testMethodChainOrderRepositorySave` |
| ✅ | Multi-Step Chain findById()->customerEmail | Verifies multi-step chain: findById() returns Order, then access customerEmail property. | `ContractTests\Tests\Chain\ChainIntegrityTest::testMultiStepChainFindByIdCustomerEmail` |
| ✅ | Property Access Chain $order->customerEmail | Verifies property access chains correctly: value (parameter/local) -> access (call) -> result (value). | `ContractTests\Tests\Chain\ChainIntegrityTest::testPropertyAccessChain` |
| ✅ | Receiver Value IDs Point to Values | Verifies every call receiver_value_id points to an existing value entry (never a call). Per schema: receiver_value_id MUST reference a value. | `ContractTests\Tests\Chain\ChainIntegrityTest::testReceiverValueIdsPointToValues` |
| ✅ | Result Values Are Kind Result | Verifies values corresponding to calls have kind=result (not parameter, local, etc.). | `ContractTests\Tests\Chain\ChainIntegrityTest::testResultValuesAreKindResult` |
| ✅ | Result Values Point Back to Source Call | Verifies every result value source_call_id equals its own id (pointing to the call that produced it). | `ContractTests\Tests\Chain\ChainIntegrityTest::testResultValuesPointBackToSourceCall` |

## Integrity Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Argument IDs Exist | Verifies every argument value_id references an existing value entry. Orphaned references indicate missing value entries for argument sources. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllArgumentIdsExist` |
| ✅ | Every Call Has Result Value | Verifies each call has a corresponding result value with the same ID. Missing result values break chain integrity as subsequent calls cannot reference the result. | `ContractTests\Tests\Integrity\DataIntegrityTest::testEveryCallHasResultValue` |
| ✅ | Full Integrity Report | Generates a complete integrity report counting all issue types: duplicate symbols, orphaned references, missing result values, type mismatches. Outputs summary to stderr for debugging. | `ContractTests\Tests\Integrity\DataIntegrityTest::testFullIntegrityReport` |
| ✅ | Receiver IDs Exist | Verifies every call receiver_value_id references an existing value entry. Orphaned references indicate missing value entries in the index. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllReceiverIdsExist` |
| ✅ | Result Types Match | Verifies result value type field matches the return_type of their source call. Type mismatches indicate incorrect type inference in the indexer. | `ContractTests\Tests\Integrity\DataIntegrityTest::testResultTypesMatch` |
| ✅ | Source Call IDs Exist | Verifies every value source_call_id references an existing call entry. Result values must point to the call that produced them. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllSourceCallIdsExist` |
| ✅ | Source Value IDs Exist | Verifies every value source_value_id references an existing value entry. Used for assignment tracking where a value derives from another value. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllSourceValueIdsExist` |

## Operator Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ⏭️ | All Operators Have Kind Type Operator | Verifies all operator kinds (coalesce, ternary, ternary_full, match) have kind_type=operator. | `ContractTests\Tests\Operator\OperatorTest::testAllOperatorsHaveKindTypeOperator` |
| ⏭️ | Coalesce Operands Reference Values | Verifies coalesce left_value_id and right_value_id point to existing values in the values array. | `ContractTests\Tests\Operator\OperatorTest::testCoalesceOperandsReferenceValues` |
| ⏭️ | Full Ternary Has All Operand IDs | Verifies full ternary ($a ? $b : $c) has condition_value_id, true_value_id, and false_value_id. | `ContractTests\Tests\Operator\OperatorTest::testFullTernaryHasAllOperandIds` |
| ⏭️ | Match Expression Arms Reference Values | Verifies match expression arm_ids array contains valid value references for each arm result. | `ContractTests\Tests\Operator\OperatorTest::testMatchExpressionArmsReferenceValues` |
| ⏭️ | Match Expression Kind Exists | Verifies match expressions are tracked with kind=match, kind_type=operator, subject_value_id, and arm_ids. | `ContractTests\Tests\Operator\OperatorTest::testMatchExpressionKindExists` |
| ⏭️ | Null Coalesce Operator Kind Exists | Verifies null coalesce operators ($a ?? $b) are tracked with kind=coalesce, kind_type=operator, left_value_id, right_value_id. | `ContractTests\Tests\Operator\OperatorTest::testNullCoalesceOperatorKindExists` |
| ⏭️ | Operators Have Result Values | Verifies operator calls have corresponding result values for data flow tracking. | `ContractTests\Tests\Operator\OperatorTest::testOperatorsHaveResultValues` |
| ⏭️ | Short Ternary Has Condition ID | Verifies short ternary ($a ?: $b) has condition_value_id. True value is the condition itself. | `ContractTests\Tests\Operator\OperatorTest::testShortTernaryHasConditionId` |
| ⏭️ | Ternary Operator Kind Exists | Verifies ternary operators ($a ? $b : $c) are tracked with kind=ternary_full, kind_type=operator, and operand IDs. | `ContractTests\Tests\Operator\OperatorTest::testTernaryOperatorKindExists` |

## Reference Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | NotificationService::notifyOrderCreated() $order - Single Value Entry | Verifies $order local has exactly ONE value entry at assignment (line 20), not entries for each of its 6 usages (lines 22, 27-34). | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testNotificationServiceOrderLocalSingleEntry` |
| ✅ | NotificationService::notifyOrderCreated() $orderId | Verifies $orderId parameter in notifyOrderCreated() has exactly one value entry and is correctly referenced when passed to findById() call. | `ContractTests\Tests\Reference\ParameterReferenceTest::testNotificationServiceOrderIdParameter` |
| ✅ | OrderRepository - No Duplicate Parameter Symbols | Verifies no parameter in OrderRepository has duplicate symbol entries (which would indicate values created at usage sites). | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderRepositoryNoDuplicateParameterSymbols` |
| ✅ | OrderRepository::save() $order - All Accesses Share Receiver | Verifies all 5 property accesses on $order (lines 31-35) have the same receiver_value_id pointing to the single parameter value at declaration. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderRepositorySaveAllAccessesShareReceiver` |
| ✅ | OrderRepository::save() $order - Single Value Entry | Verifies $order parameter has exactly ONE value entry at declaration (line 26), not 8 entries for each usage. This is the key test for the "One Value Per Declaration Rule". | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderRepositorySaveOrderParameterSingleEntry` |
| ✅ | OrderRepository::save() - Property Access Chain on $order | Verifies the 5 consecutive property accesses on $order (customerEmail, productId, quantity, status, createdAt) all share the same receiver_value_id. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderRepositoryPropertyAccessChainSharesReceiver` |
| ✅ | OrderRepository::save() - Receiver Points to Parameter | Verifies the shared receiver_value_id for $order property accesses points to a parameter value (kind=parameter), not a result or duplicate entry. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderRepositoryReceiverPointsToParameter` |
| ✅ | OrderService Constructor Parameters | Verifies promoted constructor parameters ($orderRepository, $emailSender, $inventoryChecker, $messageBus) have no duplicate symbol entries. Readonly class promoted properties are handled specially by the indexer. | `ContractTests\Tests\Reference\ParameterReferenceTest::testOrderServiceConstructorParameters` |
| ✅ | OrderService::createOrder() $input - Single Value Entry | Verifies $input parameter has exactly ONE value entry at declaration, not 4 entries for each usage (lines 29, 33-35). | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderInputParameterSingleEntry` |
| ✅ | OrderService::createOrder() $savedOrder - All Accesses Share Receiver | Verifies all property accesses on $savedOrder have the same receiver_value_id pointing to the single local value at assignment line 40. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderSavedOrderAllAccessesShareReceiver` |
| ✅ | OrderService::createOrder() $savedOrder - Single Value Entry | Verifies $savedOrder local has exactly ONE value entry at assignment (line 40), not multiple entries for each of its 8 usages. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderSavedOrderLocalSingleEntry` |
| ✅ | OrderService::createOrder() - No Duplicate Local Symbols | Verifies no local variable has duplicate symbol entries with the same @line suffix. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderNoDuplicateLocalSymbols` |
| ✅ | OrderService::getOrder() $id | Verifies $id parameter in getOrder() has exactly one value entry. Per the spec, each parameter should have a single value entry at declaration, with all usages referencing that entry. | `ContractTests\Tests\Reference\ParameterReferenceTest::testOrderServiceGetOrderIdParameter` |

## Schema Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Arguments Have Position | Verifies every argument record has a position field. Per schema ArgumentRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllArgumentsHavePosition` |
| ✅ | All Call IDs Follow Location Format | Verifies call IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallIdsFollowLocationFormat` |
| ✅ | All Call Kind Types Are Valid | Verifies every call kind_type is one of: invocation, access, operator. Per schema CallKindType enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallKindTypesAreValid` |
| ✅ | All Call Kinds Are Valid | Verifies every call kind is one of the defined enum values. Per schema CallKind enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallKindsAreValid` |
| ✅ | All Calls Have Required Fields | Verifies every call record has required fields: id, kind, kind_type, caller, location. Per schema CallRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallsHaveRequiredFields` |
| ✅ | All Value IDs Follow Location Format | Verifies value IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValueIdsFollowLocationFormat` |
| ✅ | All Value Kinds Are Valid | Verifies every value kind is one of: parameter, local, literal, constant, result. Per schema ValueKind enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValueKindsAreValid` |
| ✅ | All Values Have Required Fields | Verifies every value record has required fields: id, kind, location. Per schema ValueRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValuesHaveRequiredFields` |
| ✅ | Argument Positions Are Zero-Based | Verifies argument positions are 0-indexed and sequential for each call. | `ContractTests\Tests\Schema\SchemaValidationTest::testArgumentPositionsAreZeroBased` |
| ✅ | Call ID Matches Location | Verifies call ID is consistent with location (file:line:col format). | `ContractTests\Tests\Schema\SchemaValidationTest::testCallIdMatchesLocation` |
| ✅ | Call IDs Are Unique | Verifies all call IDs are unique within the calls array (no duplicates). | `ContractTests\Tests\Schema\SchemaValidationTest::testCallIdsAreUnique` |
| ✅ | Call Kind Type Matches Kind Category | Verifies kind_type correctly categorizes each kind. Methods/functions/constructors = invocation, property/array access = access, operators = operator. | `ContractTests\Tests\Schema\SchemaValidationTest::testCallKindTypeMatchesKindCategory` |
| ✅ | Calls Array Present and Non-Empty | Verifies calls array exists and contains entries. Required per schema. | `ContractTests\Tests\Schema\SchemaValidationTest::testCallsArrayPresentAndNonEmpty` |
| ✅ | Value ID Matches Location | Verifies value ID is consistent with location (file:line:col format). | `ContractTests\Tests\Schema\SchemaValidationTest::testValueIdMatchesLocation` |
| ✅ | Value IDs Are Unique | Verifies all value IDs are unique within the values array (no duplicates). | `ContractTests\Tests\Schema\SchemaValidationTest::testValueIdsAreUnique` |
| ✅ | Value Locations Have Required Fields | Verifies every value location has file, line, and col fields. Per schema Location definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testValueLocationsHaveRequiredFields` |
| ✅ | Values Array Present and Non-Empty | Verifies values array exists and contains entries. Required per schema. | `ContractTests\Tests\Schema\SchemaValidationTest::testValuesArrayPresentAndNonEmpty` |
| ✅ | Version Format Valid | Verifies version field matches semver pattern (e.g., "3.2"). Schema requires pattern ^[0-9]+\.[0-9]+(\.[0-9]+)?$ | `ContractTests\Tests\Schema\SchemaValidationTest::testVersionFormatValid` |

## Smoke Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Call Query Filters | Verifies CallQuery::kind() filter correctly returns only calls matching the specified kind (method) | `ContractTests\Tests\SmokeTest::testCallQueryFiltersWork` |
| ✅ | Calls Data Loaded | Verifies CallsData wrapper loaded calls.json successfully with non-empty values and calls arrays | `ContractTests\Tests\SmokeTest::testCallsDataLoaded` |
| ✅ | Index Generated | Verifies calls.json was generated by bootstrap.php during test initialization | `ContractTests\Tests\SmokeTest::testIndexWasGenerated` |
| ✅ | OrderRepository::save() $order Parameter | Critical acceptance test: Verifies $order parameter in OrderRepository::save() exists in index with kind=parameter, symbol containing ($order), and type containing Order | `ContractTests\Tests\SmokeTest::testOrderRepositorySaveParameterExists` |
| ✅ | Value Query Filters | Verifies ValueQuery::kind() filter correctly returns only values matching the specified kind (parameter) | `ContractTests\Tests\SmokeTest::testValueQueryFiltersWork` |
| ✅ | Version Present | Verifies calls.json contains a valid semver-like version field (e.g., 3.2) | `ContractTests\Tests\SmokeTest::testVersionIsPresent` |

## Valuekind Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Constant Values (If Present) | Verifies constant values have symbol, no source_call_id. Per schema: constant kind has symbol. | `ContractTests\Tests\ValueKind\ValueKindTest::testConstantValuesIfPresent` |
| ✅ | Literal Values Exist | Verifies index contains literal values (strings, integers, etc.). Per schema: literal kind, no symbol. | `ContractTests\Tests\ValueKind\ValueKindTest::testLiteralValuesExist` |
| ✅ | Literal Values No Source Call ID | Verifies literal values do not have source_call_id. Literals are not results of calls. | `ContractTests\Tests\ValueKind\ValueKindTest::testLiteralValuesNoSourceCallId` |
| ✅ | Literal Values No Symbol | Verifies literal values do not have a symbol field. Literals are anonymous values. | `ContractTests\Tests\ValueKind\ValueKindTest::testLiteralValuesNoSymbol` |
| ✅ | Local Symbol Format with @line | Verifies local symbols contain local$name@line pattern. Example: OrderService#createOrder().local$savedOrder@40 | `ContractTests\Tests\ValueKind\ValueKindTest::testLocalSymbolFormatWithLine` |
| ✅ | Local Values Have Symbol | Verifies all local values have a symbol field. Per schema: local kind has symbol with @line suffix. | `ContractTests\Tests\ValueKind\ValueKindTest::testLocalValuesHaveSymbol` |
| ✅ | Local Values Have Type From Source | Verifies local values inherit type from their source (call result or assigned value). | `ContractTests\Tests\ValueKind\ValueKindTest::testLocalValuesHaveTypeFromSource` |
| ✅ | Local Values May Have Source Call ID | Verifies local values assigned from calls have source_call_id. Per schema: local may have source_call_id or source_value_id. | `ContractTests\Tests\ValueKind\ValueKindTest::testLocalValuesMayHaveSourceCallId` |
| ✅ | Parameter Symbol Format | Verifies parameter symbols contain ($paramName) pattern. Example: OrderRepository#save().($order) | `ContractTests\Tests\ValueKind\ValueKindTest::testParameterSymbolFormat` |
| ✅ | Parameter Values Have Symbol | Verifies all parameter values have a symbol field. Per schema: parameter kind has symbol, no source_call_id. | `ContractTests\Tests\ValueKind\ValueKindTest::testParameterValuesHaveSymbol` |
| ✅ | Parameter Values Have Type | Verifies parameter values have type information when the parameter has a type declaration. | `ContractTests\Tests\ValueKind\ValueKindTest::testParameterValuesHaveType` |
| ✅ | Parameter Values No Source Call ID | Verifies parameter values do not have source_call_id. Parameters are inputs, not results of calls. | `ContractTests\Tests\ValueKind\ValueKindTest::testParameterValuesNoSourceCallId` |
| ✅ | Result Source Call Exists | Verifies every result value source_call_id points to an existing call in the calls array. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultSourceCallExists` |
| ✅ | Result Value ID Matches Source Call ID | Verifies result values have id matching their source_call_id. Per schema: result id equals the call id that produced it. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultValueIdMatchesSourceCallId` |
| ✅ | Result Values Exist | Verifies index contains result values (call results). Per schema: result kind, no symbol, always has source_call_id. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultValuesExist` |
| ✅ | Result Values Have Source Call ID | Verifies all result values have source_call_id. Per schema: result always has source_call_id. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultValuesHaveSourceCallId` |
| ✅ | Result Values Have Type From Call Return | Verifies result values have type matching the return_type of their source call. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultValuesHaveTypeFromCallReturn` |
| ✅ | Result Values No Symbol | Verifies result values do not have a symbol field. Results are anonymous intermediate values. | `ContractTests\Tests\ValueKind\ValueKindTest::testResultValuesNoSymbol` |
