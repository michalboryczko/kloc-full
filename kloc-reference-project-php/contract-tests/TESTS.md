# Contract Tests Documentation

Generated: 2026-02-05 14:39:24

## Summary

| Status | Count |
|--------|-------|
| ✅ Passed | 205 |
| ❌ Failed | 0 |
| ⏭️ Skipped | 30 |
| 💥 Error | 0 |
| **Total** | **235** |


## Argument Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Argument Parameter Symbol Present | Verifies arguments have parameter symbol linking to the callee parameter definition. | `ContractTests\Tests\Argument\ArgumentBindingTest::testArgumentParameterSymbolPresent` |
| ✅ | Argument value_expr for Complex Expressions | Verifies arguments with complex expressions (like self::$nextId++) have value_expr when value_id is null. | `ContractTests\Tests\Argument\ArgumentBindingTest::testArgumentValueExprForComplexExpressions` |
| ✅ | Order Constructor Arguments | Order constructor receives correct argument types (literal, result) | `ContractTests\Tests\Argument\ArgumentBindingTest::testOrderConstructorArguments` |
| ✅ | OrderRepository Constructor in save() | Order constructor in save() receives property access results from $order | `ContractTests\Tests\Argument\ArgumentBindingTest::testOrderRepositoryConstructorArguments` |
| ✅ | TC5: Argument Has Parameter Symbol | Verifies that arguments have parameter symbol linking to callee parameter definition. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC5ArgumentHasParameterSymbol` |
| ✅ | TC5: Argument Points to Constructor Result | Verifies that arguments from constructor results have value_id pointing to constructor result values. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC5ArgumentPointsToConstructorResult` |
| ✅ | TC5: Argument Points to Local Variable | Verifies that arguments have value_id pointing to local variable values. Per spec TC5. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC5ArgumentPointsToLocal` |
| ✅ | TC5: Argument Points to Parameter | Verifies that arguments can point to parameter values. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC5ArgumentPointsToParameter` |
| ✅ | TC5: Argument Points to Property Access Result | Verifies that arguments pointing to property access results have value_id to result values. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC5ArgumentPointsToAccessResult` |
| ✅ | checkAvailability() Receives Property Access Results | InventoryChecker receives $input property values as arguments | `ContractTests\Tests\Argument\ArgumentBindingTest::testInventoryCheckerArguments` |
| ✅ | dispatch() Receives Constructor Result | MessageBus dispatch() receives OrderCreatedMessage constructor result | `ContractTests\Tests\Argument\ArgumentBindingTest::testMessageBusDispatchArgument` |
| ✅ | findById() Receives $orderId Parameter | Argument 0 of findById() points to $orderId parameter | `ContractTests\Tests\Argument\ArgumentBindingTest::testFindByIdArgumentPointsToParameter` |
| ✅ | save() Receives $processedOrder Local | Argument 0 of save() points to $processedOrder local variable (processed via AbstractOrderProcessor) | `ContractTests\Tests\Argument\ArgumentBindingTest::testSaveArgumentPointsToProcessedOrderLocal` |
| ✅ | send() Receives customerEmail Access Result | First argument of send() points to customerEmail property access result | `ContractTests\Tests\Argument\ArgumentBindingTest::testEmailSenderReceivesCustomerEmail` |

## Callkind Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Invocation Kinds Have Arguments | Verifies all calls with kind_type=invocation have an arguments array (may be empty). | `ContractTests\Tests\CallKind\CallKindTest::testAllInvocationKindsHaveArguments` |
| ⏭️ | Array Access Kind | Verifies array access is tracked with kind=access_array. Example: self::$orders[$id]. Per schema: $arr['key']. NOTE: Array access is EXPERIMENTAL. | `ContractTests\Tests\CallKind\CallKindTest::testArrayAccessKindExists` |
| ⏭️ | Array Access on self::$orders Tracked | Verifies self::$orders[$id] array access is tracked as kind=access_array with key_value_id. NOTE: access_array is EXPERIMENTAL. | `ContractTests\Tests\CallKind\CallKindTest::testArrayAccessOnOrdersTracked` |
| ⏭️ | Array Function Calls Are Tracked | Verifies array functions (array_filter, array_map, array_keys) are tracked. | `ContractTests\Tests\CallKind\FunctionCallTest::testArrayFunctionCallsAreTracked` |
| ✅ | Constructor Call Kind Exists | Verifies index contains constructor calls (kind=constructor). Example: new Order(). Per schema: new Foo() | `ContractTests\Tests\CallKind\CallKindTest::testConstructorCallKindExists` |
| ⏭️ | Experimental Kinds Present With Flag | Verifies experimental kinds ARE present in calls.json when --experimental flag is used. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testExperimentalKindsPresentWithFlag` |
| ⏭️ | Function Call Kind | Verifies function calls are tracked with kind=function. Example: sprintf(). Per schema: func(). NOTE: Function kind is EXPERIMENTAL and requires --experimental flag. | `ContractTests\Tests\CallKind\CallKindTest::testFunctionCallKindExists` |
| ⏭️ | Function Calls Exist With Experimental Flag | Verifies function calls (sprintf, array_filter) are tracked with kind=function when --experimental is enabled. | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsExistWithExperimentalFlag` |
| ⏭️ | Function Calls Have Accurate Location | Verifies function calls have location with file, line, and col. | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsHaveAccurateLocation` |
| ⏭️ | Function Calls Have Arguments Tracked | Verifies function calls have their arguments captured. | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsHaveArgumentsTracked` |
| ⏭️ | Function Calls Have Kind Type Invocation | Verifies function calls have kind_type=invocation. | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsHaveKindTypeInvocation` |
| ⏭️ | Function Calls Have No Receiver Value ID | Verifies function calls do not have receiver_value_id (functions are not methods). | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsHaveNoReceiverValueId` |
| ⏭️ | Function Calls Have Return Type | Verifies function calls have return_type field populated. NOTE: Built-in function return types not yet implemented. | `ContractTests\Tests\CallKind\FunctionCallTest::testFunctionCallsHaveReturnType` |
| ⏭️ | Function Calls Present With Experimental Flag | Verifies function calls (kind=function) ARE present when --experimental flag is used. Tests sprintf() and other global function calls. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testFunctionCallsPresentWithExperimentalFlag` |
| ✅ | Instance Methods Have Receiver When Applicable | Verifies most instance method calls have receiver_value_id. Some calls on $this in child classes may not have it tracked. | `ContractTests\Tests\CallKind\CallKindTest::testInstanceMethodsHaveReceiver` |
| ✅ | Kind Distribution Report | Reports the distribution of call kinds in calls.json. This is an informational test that always passes but outputs statistics. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testKindDistributionReport` |
| ✅ | Method Call Kind Exists | Verifies index contains instance method calls (kind=method). Example: $this->orderRepository->save(). Per schema: $obj->method() | `ContractTests\Tests\CallKind\CallKindTest::testMethodCallKindExists` |
| ✅ | No Array Access Without Experimental Flag | Verifies array access (kind=access_array) is NOT present in default calls.json. Array access kind is experimental and requires --experimental flag. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoArrayAccessWithoutExperimentalFlag` |
| ✅ | No Coalesce Operators Without Experimental Flag | Verifies null coalesce operators (kind=coalesce) are NOT present in default calls.json. Coalesce kind is experimental and requires --experimental flag. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoCoalesceWithoutExperimentalFlag` |
| ✅ | No Deprecated Kinds Exist | Verifies deprecated kinds (access_nullsafe, method_nullsafe) do not exist in calls.json. These have been replaced by access/method with union return types. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoDeprecatedKindsExist` |
| ✅ | No Experimental Kinds in Default Output | Verifies NO experimental kinds exist in calls.json generated without --experimental flag. This is the comprehensive test for experimental filtering. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoExperimentalKindsInDefaultOutput` |
| ✅ | No Function Calls Without Experimental Flag | Verifies function calls (kind=function) are NOT present in default calls.json. Function kind is experimental and requires --experimental flag. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoFunctionCallsWithoutExperimentalFlag` |
| ✅ | No Match Expressions Without Experimental Flag | Verifies match expressions (kind=match) are NOT present in default calls.json. Match kind is experimental and requires --experimental flag. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoMatchWithoutExperimentalFlag` |
| ✅ | No Ternary Operators Without Experimental Flag | Verifies ternary operators (kind=ternary, ternary_full) are NOT present in default calls.json. Ternary kinds are experimental and require --experimental flag. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testNoTernaryWithoutExperimentalFlag` |
| ✅ | No access_nullsafe Kind Exists | Verifies calls.json contains ZERO calls with kind="access_nullsafe". This kind has been removed in favor of access with union return type. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNoAccessNullsafeKindExists` |
| ✅ | No method_nullsafe Kind Exists | Verifies calls.json contains ZERO calls with kind="method_nullsafe". This kind has been removed in favor of method with union return type. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNoMethodNullsafeKindExists` |
| ✅ | Nullsafe Boolean Method Has Union Return Type | Verifies nullsafe method call returning bool ($order?->isPending()) has union return_type "null|bool". | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafeBooleanMethodHasUnionReturnType` |
| ✅ | Nullsafe Method Call Has Union Return Type | Verifies nullsafe method call ($order?->getCustomerName()) has union return_type containing null. For method returning string, should be "null|string" union. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafeMethodCallHasUnionReturnType` |
| ✅ | Nullsafe Method Call Uses kind=method | Verifies nullsafe method call ($order?->getCustomerName()) uses kind="method" not "method_nullsafe". Per finish-mvp spec, nullsafe semantics are captured via union return type. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafeMethodCallUsesMethodKind` |
| ✅ | Nullsafe Property Access Has Union Return Type | Verifies nullsafe property access ($order?->status) has union return_type containing null. For string property, should be "null|string" union. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafePropertyAccessHasUnionReturnType` |
| ✅ | Nullsafe Property Access Uses kind=access | Verifies nullsafe property access ($order?->status) uses kind="access" not "access_nullsafe". Per finish-mvp spec, nullsafe semantics are captured via union return type. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafePropertyAccessUsesAccessKind` |
| ✅ | Nullsafe Uses Access Kind with Union Type | Verifies nullsafe property access uses kind=access (not access_nullsafe) with union return_type. Per schema: $obj?->prop uses access with return_type including null. | `ContractTests\Tests\CallKind\CallKindTest::testNullsafeUsesAccessKindWithUnionType` |
| ✅ | Order Constructor Call Tracked | Verifies new Order(...) constructor call is tracked as kind=constructor with arguments. | `ContractTests\Tests\CallKind\CallKindTest::testOrderConstructorCallTracked` |
| ✅ | OrderRepository save() Call Tracked | Verifies the $this->orderRepository->save($order) call is tracked as kind=method with correct callee. | `ContractTests\Tests\CallKind\CallKindTest::testOrderRepositorySaveCallTracked` |
| ✅ | Property Access Has Receiver When Applicable | Verifies most property access calls have receiver_value_id. $this->property in readonly classes may be handled differently. | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessHasReceiver` |
| ✅ | Property Access Kind Exists | Verifies property access is tracked with kind=access. Example: $order->customerEmail. Per schema: $obj->property | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessKindExists` |
| ✅ | Property Access on $order Tracked | Verifies $order->customerEmail property access is tracked as kind=access. | `ContractTests\Tests\CallKind\CallKindTest::testPropertyAccessOnOrderTracked` |
| ✅ | Self Static Property Access Exists | Verifies self:: static property access is tracked. Example: self::$statusLabels | `ContractTests\Tests\CallKind\StaticMethodTest::testSelfStaticPropertyAccessExists` |
| ⏭️ | Sprintf Function Calls Are Tracked | Verifies sprintf() calls are tracked with callee containing sprintf. | `ContractTests\Tests\CallKind\FunctionCallTest::testSprintfFunctionCallsAreTracked` |
| ✅ | Stable Kinds Always Present | Verifies stable kinds (access, method, constructor) are present in calls.json regardless of experimental flag. These kinds should always be generated. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testStableKindsAlwaysPresent` |
| ✅ | Static Boolean Method Has Bool Return Type | Verifies OrderStatusHelper::isTerminal() has bool return type. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticBooleanMethodHasBoolReturnType` |
| ✅ | Static Method Call Kind | Verifies static method calls are tracked with kind=method_static. Example: self::$nextId++. Per schema: Foo::method() | `ContractTests\Tests\CallKind\CallKindTest::testStaticMethodCallKindExists` |
| ✅ | Static Method Calls Have Arguments Tracked | Verifies static method calls have their arguments properly bound. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsHaveArgumentsTracked` |
| ✅ | Static Method Calls Have Kind Type Invocation | Verifies static method calls have kind_type=invocation per schema. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsHaveKindTypeInvocation` |
| ✅ | Static Method Calls Have Kind method_static | Verifies static method calls (OrderStatusHelper::getLabel()) are tracked with kind=method_static. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsHaveKindMethodStatic` |
| ✅ | Static Method Calls Have No Receiver Value ID | Verifies static method calls do not have receiver_value_id since they are called on classes, not instances. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsHaveNoReceiverValueId` |
| ✅ | Static Method Calls Have Return Type | Verifies static method calls have return_type field populated. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsHaveReturnType` |
| ✅ | Static Method Calls Reference Correct Callee Symbol | Verifies static method calls have callee pointing to the static method symbol. | `ContractTests\Tests\CallKind\StaticMethodTest::testStaticMethodCallsReferenceCorrectCalleeSymbol` |
| ✅ | Static Property Access Kind | Verifies static property access is tracked with kind=access_static. Example: self::$orders. Per schema: Foo::$property | `ContractTests\Tests\CallKind\CallKindTest::testStaticPropertyAccessKindExists` |
| ⏭️ | sprintf Function Call Tracked | Verifies sprintf() function call is tracked as kind=function. NOTE: Function kind is EXPERIMENTAL and requires --experimental flag. | `ContractTests\Tests\CallKind\CallKindTest::testSprintfFunctionCallTracked` |

## Chain Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Chains Terminate at Valid Source | Verifies tracing any chain backwards terminates at a parameter, local, literal, or constant (not orphaned). | `ContractTests\Tests\Chain\ChainIntegrityTest::testAllChainsTerminateAtValidSource` |
| ✅ | Argument Value IDs Point to Values | Verifies every argument value_id points to an existing value entry (never a call). Per schema: argument value_id MUST reference a value. | `ContractTests\Tests\Chain\ChainIntegrityTest::testArgumentValueIdsPointToValues` |
| ✅ | Contact and Address Accesses Share Customer Receiver | Verifies that $customer->contact and $customer->address property accesses both have the same receiver_value_id (the $customer local variable). | `ContractTests\Tests\Chain\NestedChainTest::testContactAndAddressAccessesShareCustomerReceiver` |
| ✅ | Deep Chain Walk from Response to Entity | Verifies complete chain integrity: walks the full chain from a nested property access backwards through all receivers to the original source, verifying each link exists. | `ContractTests\Tests\Chain\NestedChainTest::testDeepChainWalkFromResponseToEntity` |
| ✅ | Direct Nested Access in Constructor Arguments | Verifies that getCustomerDetails with direct nested access in constructor (email: $customer->contact->email) correctly tracks the chain. | `ContractTests\Tests\Chain\NestedChainTest::testDirectNestedAccessInConstructorArguments` |
| ✅ | Every Call Has Corresponding Result Value | Verifies each call has a result value with the same ID in the values array. Per schema: calls and result values share the same ID. | `ContractTests\Tests\Chain\ChainIntegrityTest::testEveryCallHasResultValue` |
| ✅ | Full Flow: Controller Response to Entity Property | Verifies the complete data flow from CustomerController creating CustomerResponse, through CustomerService returning CustomerOutput, back to the Entity nested properties. Tests that $output->street in Controller traces to CustomerOutput, which traces to $street local in Service, which traces to $customer->address->street. | `ContractTests\Tests\Chain\NestedChainTest::testFullFlowControllerToEntity` |
| ✅ | Full Flow: Service Output to Entity Nested Chain | Verifies that CustomerOutput properties in Service can be traced back through nested entity access chains: $output.street <- $street local <- $customer->address->street <- $customer local <- repository call. | `ContractTests\Tests\Chain\NestedChainTest::testFullFlowServiceOutputToEntityChain` |
| ✅ | Integration: Full Chain Trace from Argument to Source | Verifies complete data flow can be traced: argument -> value -> (result -> call)* -> parameter/local. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testIntegrationFullChainTrace` |
| ✅ | Method Call on Address Nested Object | Verifies $customer->address->getFullAddress() has correct chain structure. | `ContractTests\Tests\Chain\NestedChainTest::testMethodCallOnAddressNestedObject` |
| ✅ | Method Call on Nested Object Result | Verifies $customer->contact->getFormattedEmail() has correct chain: getFormattedEmail() receiver points to contact access result, contact access receiver points to $customer. | `ContractTests\Tests\Chain\NestedChainTest::testMethodCallOnNestedObjectResult` |
| ✅ | Method Chain $this->orderRepository->save() | Verifies the chain $this->orderRepository->save() is traceable: $this (value) -> orderRepository (access) -> result (value) -> save (method) -> result (value). | `ContractTests\Tests\Chain\ChainIntegrityTest::testMethodChainOrderRepositorySave` |
| ✅ | Multi-Step Chain findById()->customerEmail | Verifies multi-step chain: findById() returns Order, then access customerEmail property. | `ContractTests\Tests\Chain\ChainIntegrityTest::testMultiStepChainFindByIdCustomerEmail` |
| ✅ | Multiple Chains in Same Expression Share Receivers | Verifies that in getCustomerSummary, both $customer->contact->email and $customer->address->city share the same $customer receiver. | `ContractTests\Tests\Chain\NestedChainTest::testMultipleChainsInSameExpressionShareReceivers` |
| ✅ | Multiple Nullsafe Accesses Share Receiver | Verifies multiple nullsafe property accesses on same variable share the same receiver_value_id. In getOrderSummary(): $order?->id, $order?->customerEmail, $order?->status should all point to same $order value. | `ContractTests\Tests\CallKind\NullsafeKindTest::testMultipleNullsafeAccessesShareReceiver` |
| ✅ | Nested Address Property Accesses Share Receiver | Verifies $customer->address->street and $customer->address->city both have the same receiver_value_id for the address access. | `ContractTests\Tests\Chain\NestedChainTest::testNestedAddressPropertyAccessesShareReceiver` |
| ✅ | Nested Contact Property Accesses Share Receiver | Verifies $customer->contact->email and $customer->contact->phone both have the same receiver_value_id for the contact access (pointing to the result of $customer->contact). | `ContractTests\Tests\Chain\NestedChainTest::testNestedContactPropertyAccessesShareReceiver` |
| ✅ | No Orphaned References in Nested Chains | Verifies that all receiver_value_id and source_call_id references in CustomerService nested chains point to existing entries. | `ContractTests\Tests\Chain\NestedChainTest::testNoOrphanedReferencesInNestedChains` |
| ✅ | Nullsafe Access Has Receiver Value | Verifies nullsafe property access has receiver_value_id pointing to a valid value in the values array. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafeAccessHasReceiverValue` |
| ✅ | Nullsafe Access Result Value Exists | Verifies nullsafe property access creates a result value with source_call_id pointing to the access call. | `ContractTests\Tests\CallKind\NullsafeKindTest::testNullsafeAccessResultValueExists` |
| ✅ | Property Access Chain $order->customerEmail | Verifies property access chains correctly: value (parameter/local) -> access (call) -> result (value). | `ContractTests\Tests\Chain\ChainIntegrityTest::testPropertyAccessChain` |
| ✅ | Receiver Value IDs Point to Values | Verifies every call receiver_value_id points to an existing value entry (never a call). Per schema: receiver_value_id MUST reference a value. | `ContractTests\Tests\Chain\ChainIntegrityTest::testReceiverValueIdsPointToValues` |
| ✅ | Result Values Are Kind Result | Verifies values corresponding to calls have kind=result (not parameter, local, etc.). | `ContractTests\Tests\Chain\ChainIntegrityTest::testResultValuesAreKindResult` |
| ✅ | Result Values Point Back to Source Call | Verifies every result value source_call_id equals its own id (pointing to the call that produced it). | `ContractTests\Tests\Chain\ChainIntegrityTest::testResultValuesPointBackToSourceCall` |
| ✅ | TC2: Method Call Has receiver_value_id to Property Access Result | Verifies that method calls on properties have receiver_value_id pointing to the property access result value. Per spec TC2, method calls via property need proper linkage. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC2MethodCallHasReceiverFromPropertyAccess` |
| ✅ | TC2: Property Access Creates Result Value | Verifies that property access calls create corresponding result values with matching IDs. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC2PropertyAccessCreatesResultValue` |
| ✅ | TC3: Chain Traversal Value->Access->Result->Method->Result | Verifies the full chain $this->orderRepository->save() can be traced: value -> access -> result -> method -> result. Per spec TC3, chains must be reconstructable. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC3ChainTraversalPropertyToMethod` |
| ✅ | TC3: Local Variable Links to Method Result | Verifies that local variables assigned from method calls have source_call_id pointing to the method call. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC3LocalVariableLinksToMethodResult` |
| ✅ | TC3: Nested Property Chain CustomerService | Verifies nested property chains like $customer->contact->email can be traced. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC3NestedPropertyChain` |
| ✅ | TC3: Property Access Uses Local as Receiver | Verifies that property accesses on local variables use the local variable value as receiver. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC3PropertyAccessUsesLocalAsReceiver` |
| ✅ | Value Flow from Entity to Service DTO | Verifies we can trace the value of $street from CustomerOutput constructor argument back through the chain: argument -> $street local -> address->street access -> address access -> $customer. | `ContractTests\Tests\Chain\NestedChainTest::testValueFlowFromEntityToResponse` |

## Combined Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Constructor calls have both entries | Verifies new Order() appears in both calls.json (kind=constructor) and SCIP (reference occurrence). | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testConstructorCallsHaveBothEntries` |
| ✅ | Method calls have SCIP occurrences | Verifies method calls in calls.json (kind=method) have corresponding reference occurrences in SCIP. | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testMethodCallsHaveScipOccurrences` |
| ✅ | Property accesses have SCIP occurrences | Verifies property accesses in calls.json (kind=access) have corresponding reference occurrences in SCIP. | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testPropertyAccessesHaveScipOccurrences` |
| ✅ | Source files are consistently indexed | Verifies the same source files appear in both calls.json scope and SCIP documents. | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testSourceFilesAreConsistentlyIndexed` |
| ✅ | Symbol formats are compatible | Verifies symbol formats between calls.json callee and SCIP symbols are compatible after normalization. | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testSymbolFormatsAreCompatible` |
| ✅ | Types correspond to SCIP symbols | Verifies value types in calls.json reference valid SCIP class symbols for project classes. | `ContractTests\Tests\Combined\Consistency\ConsistencyTest::testTypesCorrespondToScipSymbols` |

## Integrity Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Argument value_ids Point to Values | Verifies every argument value_id references an existing value entry in the values array. This ensures type lookup is possible. | `ContractTests\Tests\Argument\ArgumentSchemaTest::testAllArgumentValueIdsPointToValues` |
| ✅ | Argument IDs Exist | Verifies every argument value_id references an existing value entry. Orphaned references indicate missing value entries for argument sources. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllArgumentIdsExist` |
| ✅ | Every Call Has Result Value | Verifies each call has a corresponding result value with the same ID. Missing result values break chain integrity as subsequent calls cannot reference the result. | `ContractTests\Tests\Integrity\DataIntegrityTest::testEveryCallHasResultValue` |
| ✅ | Full Integrity Report | Generates a complete integrity report counting all issue types: duplicate symbols, orphaned references, missing result values, type mismatches. Outputs summary to stderr for debugging. | `ContractTests\Tests\Integrity\DataIntegrityTest::testFullIntegrityReport` |
| ✅ | Receiver IDs Exist | Verifies every call receiver_value_id references an existing value entry. Orphaned references indicate missing value entries in the index. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllReceiverIdsExist` |
| ✅ | Result Types Match | Verifies result value type field matches the return_type of their source call. Type mismatches indicate incorrect type inference in the indexer. | `ContractTests\Tests\Integrity\DataIntegrityTest::testResultTypesMatch` |
| ✅ | Source Call IDs Exist | Verifies every value source_call_id references an existing call entry. Result values must point to the call that produced them. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllSourceCallIdsExist` |
| ✅ | Source Value IDs Exist | Verifies every value source_value_id references an existing value entry. Used for assignment tracking where a value derives from another value. | `ContractTests\Tests\Integrity\DataIntegrityTest::testAllSourceValueIdsExist` |
| ✅ | Type Lookup Via value_id Works | Verifies that for arguments with value_id, the type can be looked up via values[value_id].type. This is the replacement for the removed value_type field. | `ContractTests\Tests\Argument\ArgumentSchemaTest::testTypeLookupViaValueIdWorks` |

## Operator Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ⏭️ | All Operators Have Kind Type Operator | Verifies all operator kinds (coalesce, ternary, ternary_full, match) have kind_type=operator. | `ContractTests\Tests\Operator\OperatorTest::testAllOperatorsHaveKindTypeOperator` |
| ⏭️ | Coalesce Operands Reference Values | Verifies coalesce left_value_id and right_value_id point to existing values in the values array. | `ContractTests\Tests\Operator\OperatorTest::testCoalesceOperandsReferenceValues` |
| ⏭️ | Coalesce Return Type Removes Null | Verifies coalesce return_type correctly removes null from left operand. For ($nullable ?? $default), if left is T|null and right is T, result is T (not T|null). | `ContractTests\Tests\Operator\OperatorTest::testCoalesceReturnTypeRemovesNull` |
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
| ✅ | OrderService::createOrder() $savedOrder - All Accesses Share Receiver | Verifies all property accesses on $savedOrder have the same receiver_value_id pointing to the single local value at assignment line 45. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderSavedOrderAllAccessesShareReceiver` |
| ✅ | OrderService::createOrder() $savedOrder - Single Value Entry | Verifies $savedOrder local has exactly ONE value entry at assignment (line 45), not multiple entries for each of its 8 usages. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderSavedOrderLocalSingleEntry` |
| ✅ | OrderService::createOrder() - No Duplicate Local Symbols | Verifies no local variable has duplicate symbol entries with the same @line suffix. | `ContractTests\Tests\Reference\OneValuePerDeclarationTest::testOrderServiceCreateOrderNoDuplicateLocalSymbols` |
| ✅ | OrderService::getOrder() $id | Verifies $id parameter in getOrder() has exactly one value entry. Per the spec, each parameter should have a single value entry at declaration, with all usages referencing that entry. | `ContractTests\Tests\Reference\ParameterReferenceTest::testOrderServiceGetOrderIdParameter` |
| ✅ | TC1: Local Variable Assigned from Method Has Type | Verifies that local variables assigned from method calls can have type information via source_call_id linkage. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC1LocalVariableFromMethodHasType` |
| ✅ | TC1: Property Type Hint Creates Typed Parameter Value | Verifies that constructor parameters with type hints have type information in the value entry. Per spec TC1, property type hints should create values with types. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC1PropertyTypeHintCreatesTypedValue` |
| ✅ | TC4: Multiple Calls in Same Scope Have Unique IDs | Verifies that multiple method calls in the same scope are NOT collapsed - each has a unique ID. Per spec TC4. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC4MultipleCallsHaveUniqueIds` |
| ✅ | TC4: Multiple Property Accesses Share Receiver | Verifies that multiple property accesses on the same variable all share the same receiver_value_id. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC4MultipleAccessesShareReceiver` |
| ✅ | TC4: Same Property Multiple Times Has Multiple Entries | Verifies that accessing the same property multiple times creates multiple call entries. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testTC4SamePropertyMultipleEntriesNotCollapsed` |

## Returntype Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Builtin Return Types Have Correct Format | Verifies builtin types (string, int, bool) use scip-php php builtin format. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testBuiltinReturnTypesHaveCorrectFormat` |
| ⏭️ | Coalesce Operator Has Return Type Without Null | Verifies coalesce ($a ?? $b) has return_type without null (null is handled by fallback). | `ContractTests\Tests\ReturnType\ReturnTypeTest::testCoalesceOperatorHasReturnTypeWithoutNull` |
| ✅ | Constructor Calls Have Return Type | Verifies new Class() calls have return_type matching the constructed class. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testConstructorCallsHaveReturnType` |
| ✅ | Method Calls Have Return Type | Verifies method calls have return_type field populated when method has declared return type. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testMethodCallsHaveReturnType` |
| ✅ | Order Constructor Returns Order Type | Verifies new Order() has return_type containing Order. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testOrderConstructorReturnsOrderType` |
| ✅ | Order Status Property Access Has String Type | Verifies $order->status access has string return_type. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testOrderStatusPropertyAccessHasStringType` |
| ✅ | OrderRepository findById Returns Nullable Order | Verifies findById() call has return_type containing Order and null union. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testOrderRepositoryFindByIdReturnsNullableOrder` |
| ✅ | OrderRepository save Returns Order | Verifies save() call has return_type containing Order. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testOrderRepositorySaveReturnsOrder` |
| ✅ | Property Access Has Return Type | Verifies property access calls have return_type matching property type. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testPropertyAccessHasReturnType` |
| ✅ | Return Types Use SCIP Symbol Format | Verifies return_type fields use proper SCIP symbol format (scip-php ...). | `ContractTests\Tests\ReturnType\ReturnTypeTest::testReturnTypesUseScipSymbolFormat` |
| ⏭️ | Ternary Operator Has Return Type | Verifies ternary ($a ? $b : $c) has return_type as union of branch types. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testTernaryOperatorHasReturnType` |
| ✅ | Union Return Types Have Correct Format | Verifies union types use scip-php synthetic union format. | `ContractTests\Tests\ReturnType\ReturnTypeTest::testUnionReturnTypesHaveCorrectFormat` |

## Schema Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | All Arguments Have Position | Verifies every argument record has a position field. Per schema ArgumentRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllArgumentsHavePosition` |
| ✅ | All Call IDs Follow Location Format | Verifies call IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallIdsFollowLocationFormat` |
| ✅ | All Call Kind Types Are Valid | Verifies every call kind_type is one of: invocation, access, operator. Per schema CallKindType enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallKindTypesAreValid` |
| ✅ | All Call Kinds Are Valid | Verifies every call kind is one of the defined enum values. Per schema CallKind enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallKindsAreValid` |
| ✅ | All Call Kinds Are Valid | Verifies every call in calls.json has a kind that is either stable, experimental, or deprecated. No unknown kinds should exist. | `ContractTests\Tests\CallKind\ExperimentalKindTest::testAllCallKindsAreValid` |
| ✅ | All Calls Have Required Fields | Verifies every call record has required fields: id, kind, kind_type, caller, location. Per schema CallRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllCallsHaveRequiredFields` |
| ✅ | All Value IDs Follow Location Format | Verifies value IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValueIdsFollowLocationFormat` |
| ✅ | All Value Kinds Are Valid | Verifies every value kind is one of: parameter, local, literal, constant, result. Per schema ValueKind enum. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValueKindsAreValid` |
| ✅ | All Values Have Required Fields | Verifies every value record has required fields: id, kind, location. Per schema ValueRecord definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testAllValuesHaveRequiredFields` |
| ✅ | Argument Fields Match Schema | Verifies argument records contain only the expected fields: position (required), parameter, value_id, value_expr. No extra fields allowed. | `ContractTests\Tests\Argument\ArgumentSchemaTest::testArgumentFieldsMatchSchema` |
| ✅ | Argument Positions Are Zero-Based | Verifies argument positions are 0-indexed and sequential for each call. | `ContractTests\Tests\Schema\SchemaValidationTest::testArgumentPositionsAreZeroBased` |
| ✅ | Arguments Have No value_type Field | Verifies argument records do NOT contain a value_type field. Per finish-mvp spec, consumers should use values[value_id].type instead. | `ContractTests\Tests\Argument\ArgumentSchemaTest::testArgumentsHaveNoValueTypeField` |
| ✅ | Call ID Matches Location | Verifies call ID is consistent with location (file:line:col format). | `ContractTests\Tests\Schema\SchemaValidationTest::testCallIdMatchesLocation` |
| ✅ | Call IDs Are Unique | Verifies all call IDs are unique within the calls array (no duplicates). | `ContractTests\Tests\Schema\SchemaValidationTest::testCallIdsAreUnique` |
| ✅ | Call Kind Type Matches Kind Category | Verifies kind_type correctly categorizes each kind. Methods/functions/constructors = invocation, property/array access = access, operators = operator. | `ContractTests\Tests\Schema\SchemaValidationTest::testCallKindTypeMatchesKindCategory` |
| ✅ | Calls Array Present and Non-Empty | Verifies calls array exists and contains entries. Required per schema. | `ContractTests\Tests\Schema\SchemaValidationTest::testCallsArrayPresentAndNonEmpty` |
| ✅ | Value ID Matches Location | Verifies value ID is consistent with location (file:line:col format). | `ContractTests\Tests\Schema\SchemaValidationTest::testValueIdMatchesLocation` |
| ✅ | Value IDs Are Unique | Verifies all value IDs are unique within the values array (no duplicates). | `ContractTests\Tests\Schema\SchemaValidationTest::testValueIdsAreUnique` |
| ✅ | Value Locations Have Required Fields | Verifies every value location has file, line, and col fields. Per schema Location definition. | `ContractTests\Tests\Schema\SchemaValidationTest::testValueLocationsHaveRequiredFields` |
| ✅ | Values Array Present and Non-Empty | Verifies values array exists and contains entries. Required per schema. | `ContractTests\Tests\Schema\SchemaValidationTest::testValuesArrayPresentAndNonEmpty` |
| ✅ | Version Format Valid | Verifies version field matches semver pattern (e.g., "3.2"). Schema requires pattern ^[0-9]+\.[0-9]+(\.[0-9]+)?$ | `ContractTests\Tests\Schema\SchemaValidationTest::testVersionFormatValid` |

## Scip Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Abstract class symbol exists | Verifies AbstractOrderProcessor has symbol definition in SCIP index. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testAbstractClassSymbolExists` |
| ✅ | Abstract method implementation tracked | Verifies child class implementing abstract parent method has proper symbol relationship. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testAbstractMethodImplementationTracked` |
| ✅ | All classes have definitions | Verifies every class in the project has at least one definition occurrence in SCIP. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testAllClassesHaveDefinitionOccurrences` |
| ✅ | All implementing classes have relationships | Verifies every class implementing an interface has proper implementation relationship in SCIP. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testAllImplementingClassesHaveRelationships` |
| ✅ | Child class symbol exists | Verifies StandardOrderProcessor class has symbol definition in SCIP index. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testChildClassSymbolExists` |
| ✅ | Class extends abstract class creates relationship | Verifies StandardOrderProcessor has relationship with is_reference to AbstractOrderProcessor. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testClassExtendsAbstractCreatesRelationship` |
| ✅ | Class has definition occurrence | Verifies Order class symbol has a definition occurrence with role=Definition in SCIP index. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testClassDefinitionOccurrenceExists` |
| ✅ | Class implements interface creates relationship | Verifies EmailSender has relationship with kind=implementation to EmailSenderInterface. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testClassImplementsInterfaceCreatesRelationship` |
| ✅ | Constructor call creates reference | Verifies new Order() constructor calls create reference occurrences for the class. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testConstructorCallCreatesReference` |
| ✅ | Constructor has definition | Verifies class constructors have definition occurrences. Tests Order.__construct(). | `ContractTests\Tests\Scip\Symbol\SymbolTest::testConstructorDefinitionExists` |
| ✅ | Definition occurrence has correct location | Verifies Order class definition is in src/Entity/Order.php with valid location. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testDefinitionOccurrenceHasCorrectLocation` |
| ✅ | Every symbol has at least one occurrence | Verifies every symbol in the SCIP index has at least one occurrence (definition or reference). | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testEverySymbolHasOccurrence` |
| ✅ | Extends relationship distinct from implements | Verifies extends uses is_reference while implements uses is_implementation. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testExtendsDistinctFromImplements` |
| ✅ | Interface has definition occurrence | Verifies interfaces have definition occurrences in their source files. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testInterfaceHasDefinitionOccurrence` |
| ✅ | Interface method has definition | Verifies EmailSenderInterface.send() has a definition occurrence in SCIP index. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testInterfaceMethodDefinitionExists` |
| ✅ | Interface symbols exist | Verifies interfaces (EmailSenderInterface, InventoryCheckerInterface) have symbol definitions. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testInterfaceSymbolsExist` |
| ✅ | Interface type hint creates SCIP occurrence | Verifies interface type hints create reference occurrences. Tests OrderService.$emailSender typed as EmailSenderInterface. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testInterfaceTypeHintCreatesOccurrence` |
| ✅ | InventoryChecker implements interface relationship | Verifies InventoryChecker has implementation relationship to InventoryCheckerInterface. | `ContractTests\Tests\Scip\Inheritance\InheritanceTest::testInventoryCheckerImplementsInterface` |
| ✅ | Method call creates reference occurrence | Verifies method calls like $this->orderRepository->save() create reference occurrences. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testMethodCallCreatesReferenceOccurrence` |
| ✅ | Method has definition occurrence | Verifies Order.getCustomerName() has a definition occurrence in SCIP index. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testMethodDefinitionOccurrenceExists` |
| ✅ | Multiple type hints in same method | Verifies both parameter and return types create separate reference occurrences. Tests createOrder(CreateOrderInput): OrderOutput. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testMultipleTypeHintsInSameMethod` |
| ✅ | Nullable type hint creates SCIP occurrence | Verifies nullable return types (?Order) create reference occurrences for the base type. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testNullableTypeHintCreatesOccurrence` |
| ✅ | Occurrence file paths are consistent | Verifies all occurrences have valid relative file paths. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testOccurrenceFilePathsAreConsistent` |
| ✅ | Occurrence roles are correct | Verifies definition occurrences have the Definition role bit set. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testOccurrenceRolesAreCorrect` |
| ✅ | Parameter type hint creates SCIP occurrence | Verifies method parameter type hints create reference occurrences. Tests OrderRepository.save() parameter typed as Order. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testParameterTypeHintCreatesOccurrence` |
| ✅ | Property access creates reference occurrence | Verifies property accesses like $savedOrder->customerEmail create reference occurrences. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testPropertyAccessCreatesReferenceOccurrence` |
| ✅ | Property has definition occurrence | Verifies Order.$customerEmail has a definition occurrence in SCIP index. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testPropertyDefinitionOccurrenceExists` |
| ✅ | Property type hint creates SCIP occurrence | Verifies typed properties create reference occurrences in SCIP index. Tests OrderService.$orderRepository typed as OrderRepository. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testPropertyTypeHintCreatesOccurrence` |
| ✅ | References tracked across files | Verifies Order class has reference occurrences in files that use it as type hints. | `ContractTests\Tests\Scip\Occurrence\OccurrenceTest::testReferencesTrackedAcrossFiles` |
| ✅ | Return type hint creates SCIP occurrence | Verifies method return type hints create reference occurrences. Tests OrderRepository.save() return type Order. | `ContractTests\Tests\Scip\TypeHint\TypeHintTest::testReturnTypeHintCreatesOccurrence` |
| ✅ | Symbol count is reasonable | Verifies the SCIP index contains expected number of symbols for the project size. | `ContractTests\Tests\Scip\Symbol\SymbolTest::testSymbolCountIsReasonable` |

## Smoke Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ✅ | Call Query Filters | Verifies CallQuery::kind() filter correctly returns only calls matching the specified kind (method) | `ContractTests\Tests\SmokeTest::testCallQueryFiltersWork` |
| ✅ | Calls Data Loaded | Verifies CallsData wrapper loaded calls.json successfully with non-empty values and calls arrays | `ContractTests\Tests\SmokeTest::testCallsDataLoaded` |
| ✅ | Index Generated | Verifies calls.json was generated by bootstrap.php during test initialization | `ContractTests\Tests\SmokeTest::testIndexWasGenerated` |
| ✅ | OrderRepository::save() $order Parameter | Critical acceptance test: Verifies $order parameter in OrderRepository::save() exists in index with kind=parameter, symbol containing ($order), and type containing Order | `ContractTests\Tests\SmokeTest::testOrderRepositorySaveParameterExists` |
| ✅ | Summary: All TC Requirements Traceable | Summary test verifying the spec requirements are structurally supported in calls.json. | `ContractTests\Tests\UsageFlow\UsageFlowTrackingTest::testSummaryAllRequirementsTraceable` |
| ✅ | Value Query Filters | Verifies ValueQuery::kind() filter correctly returns only values matching the specified kind (parameter) | `ContractTests\Tests\SmokeTest::testValueQueryFiltersWork` |
| ✅ | Version Present | Verifies calls.json contains a valid semver-like version field (e.g., 3.2) | `ContractTests\Tests\SmokeTest::testVersionIsPresent` |

## Valuekind Tests

| Status | Test Name | Description | Code Ref |
|--------|-----------|-------------|----------|
| ⏭️ | Boolean Literals Have Type Information | Verifies boolean literal values (true, false) have type containing bool. | `ContractTests\Tests\ValueKind\LiteralValueTest::testBooleanLiteralsHaveTypeInformation` |
| ✅ | Constant Values (If Present) | Verifies constant values have symbol, no source_call_id. Per schema: constant kind has symbol. | `ContractTests\Tests\ValueKind\ValueKindTest::testConstantValuesIfPresent` |
| ⏭️ | Integer Literals Have Type Information | Verifies integer literal values have type containing int. | `ContractTests\Tests\ValueKind\LiteralValueTest::testIntegerLiteralsHaveTypeInformation` |
| ✅ | Literal Arguments Have Value Expression | Verifies literals passed as arguments have value_expr capturing the literal text. | `ContractTests\Tests\ValueKind\LiteralValueTest::testLiteralArgumentsHaveValueExpression` |
| ✅ | Literal Values Exist | Verifies index contains literal values (strings, integers, etc.). Per schema: literal kind, no symbol. | `ContractTests\Tests\ValueKind\ValueKindTest::testLiteralValuesExist` |
| ✅ | Literal Values Exist | Verifies literal values (strings, numbers) are tracked with kind=literal in values array. | `ContractTests\Tests\ValueKind\LiteralValueTest::testLiteralValuesExist` |
| ✅ | Literal Values Have Required Fields | Verifies literal values have id, kind, and location fields. | `ContractTests\Tests\ValueKind\LiteralValueTest::testLiteralValuesHaveRequiredFields` |
| ⏭️ | Literal Values Have Type Field | Verifies literal values have type field indicating the literal type. NOTE: Literal type tracking not yet fully implemented. | `ContractTests\Tests\ValueKind\LiteralValueTest::testLiteralValuesHaveTypeField` |
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
| ✅ | String Literal Arguments Have Quoted Value Expression | Verifies string literals in arguments preserve quotes in value_expr. | `ContractTests\Tests\ValueKind\LiteralValueTest::testStringLiteralArgumentsHaveQuotedValueExpression` |
| ⏭️ | String Literals Have Type Information | Verifies string literal values have type containing string. NOTE: Literal type tracking not yet fully implemented. | `ContractTests\Tests\ValueKind\LiteralValueTest::testStringLiteralsHaveTypeInformation` |
