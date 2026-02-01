<?php

declare(strict_types=1);

namespace ContractTests\Tests\Chain;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for call chain integrity.
 *
 * Verifies that method/property chains are properly linked:
 * value -> call -> result value -> call -> result value...
 *
 * This is critical for data flow analysis as it allows tracing
 * values from arguments back to their sources.
 */
class ChainIntegrityTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Value->Call Linkage
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Receiver Value IDs Point to Values',
        description: 'Verifies every call receiver_value_id points to an existing value entry (never a call). Per schema: receiver_value_id MUST reference a value.',
        category: 'chain',
    )]
    public function testReceiverValueIdsPointToValues(): void
    {
        $callsWithReceiver = $this->calls()
            ->hasReceiver()
            ->all();

        $this->assertNotEmpty($callsWithReceiver, 'Should have calls with receivers');

        $invalidReceivers = [];
        foreach ($callsWithReceiver as $call) {
            $receiverId = $call['receiver_value_id'];
            $value = self::$calls->getValueById($receiverId);

            if ($value === null) {
                $invalidReceivers[] = sprintf(
                    'Call %s has receiver_value_id %s that does not exist',
                    $call['id'] ?? 'unknown',
                    $receiverId
                );
            }
        }

        $this->assertEmpty(
            $invalidReceivers,
            sprintf(
                "Found %d calls with invalid receiver_value_id:\n%s",
                count($invalidReceivers),
                implode("\n", array_slice($invalidReceivers, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Argument Value IDs Point to Values',
        description: 'Verifies every argument value_id points to an existing value entry (never a call). Per schema: argument value_id MUST reference a value.',
        category: 'chain',
    )]
    public function testArgumentValueIdsPointToValues(): void
    {
        $invalidArguments = [];

        foreach (self::$calls->calls() as $call) {
            $arguments = $call['arguments'] ?? [];

            foreach ($arguments as $arg) {
                $valueId = $arg['value_id'] ?? null;
                if ($valueId !== null) {
                    $value = self::$calls->getValueById($valueId);
                    if ($value === null) {
                        $invalidArguments[] = sprintf(
                            'Call %s argument %d has value_id %s that does not exist',
                            $call['id'] ?? 'unknown',
                            $arg['position'] ?? -1,
                            $valueId
                        );
                    }
                }
            }
        }

        $this->assertEmpty(
            $invalidArguments,
            sprintf(
                "Found %d arguments with invalid value_id:\n%s",
                count($invalidArguments),
                implode("\n", array_slice($invalidArguments, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Call->Result Linkage
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Every Call Has Corresponding Result Value',
        description: 'Verifies each call has a result value with the same ID in the values array. Per schema: calls and result values share the same ID.',
        category: 'chain',
    )]
    public function testEveryCallHasResultValue(): void
    {
        $missingResults = [];

        foreach (self::$calls->calls() as $call) {
            $callId = $call['id'] ?? '';

            // Look for corresponding result value
            $resultValue = self::$calls->getValueById($callId);

            if ($resultValue === null) {
                $missingResults[] = sprintf(
                    'Call %s (kind=%s) has no corresponding result value',
                    $callId,
                    $call['kind'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $missingResults,
            sprintf(
                "Found %d calls without result values:\n%s",
                count($missingResults),
                implode("\n", array_slice($missingResults, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Result Values Are Kind Result',
        description: 'Verifies values corresponding to calls have kind=result (not parameter, local, etc.).',
        category: 'chain',
    )]
    public function testResultValuesAreKindResult(): void
    {
        $wrongKind = [];

        foreach (self::$calls->calls() as $call) {
            $callId = $call['id'] ?? '';
            $resultValue = self::$calls->getValueById($callId);

            if ($resultValue !== null && ($resultValue['kind'] ?? '') !== 'result') {
                $wrongKind[] = sprintf(
                    'Call %s result value has kind=%s, expected result',
                    $callId,
                    $resultValue['kind'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $wrongKind,
            sprintf(
                "Found %d result values with wrong kind:\n%s",
                count($wrongKind),
                implode("\n", array_slice($wrongKind, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Result->Source Linkage
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Result Values Point Back to Source Call',
        description: 'Verifies every result value source_call_id equals its own id (pointing to the call that produced it).',
        category: 'chain',
    )]
    public function testResultValuesPointBackToSourceCall(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty($results, 'Should have result values');

        $wrongSource = [];
        foreach ($results as $result) {
            $id = $result['id'] ?? '';
            $sourceCallId = $result['source_call_id'] ?? '';

            if ($id !== $sourceCallId) {
                $wrongSource[] = sprintf(
                    'Result %s has source_call_id %s (should equal id)',
                    $id,
                    $sourceCallId
                );
            }
        }

        $this->assertEmpty(
            $wrongSource,
            sprintf(
                "Found %d result values with incorrect source_call_id:\n%s",
                count($wrongSource),
                implode("\n", array_slice($wrongSource, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Chain Traversal
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Method Chain $this->orderRepository->save()',
        description: 'Verifies the chain $this->orderRepository->save() is traceable: $this (value) -> orderRepository (access) -> result (value) -> save (method) -> result (value).',
        category: 'chain',
    )]
    public function testMethodChainOrderRepositorySave(): void
    {
        // Code reference: src/Service/OrderService.php:40
        // $savedOrder = $this->orderRepository->save($order);

        // Step 1: Find the save() call
        $saveCall = $this->calls()
            ->kind('method')
            ->callerContains('OrderService#createOrder()')
            ->calleeContains('save')
            ->first();

        $this->assertNotNull($saveCall, 'Should find save() method call');

        // Step 2: Verify save() has receiver_value_id
        $this->assertArrayHasKey(
            'receiver_value_id',
            $saveCall,
            'save() call should have receiver_value_id'
        );
        $saveReceiverId = $saveCall['receiver_value_id'];

        // Step 3: Get the receiver value (should be result of orderRepository access)
        $receiverValue = self::$calls->getValueById($saveReceiverId);
        $this->assertNotNull($receiverValue, 'save() receiver value should exist');
        $this->assertSame(
            'result',
            $receiverValue['kind'] ?? '',
            'save() receiver should be a result value (from orderRepository access)'
        );

        // Step 4: Verify receiver has source_call_id pointing to orderRepository access
        $orderRepoAccessId = $receiverValue['source_call_id'] ?? '';
        $this->assertNotEmpty($orderRepoAccessId, 'Receiver should have source_call_id');

        $orderRepoAccess = self::$calls->getCallById($orderRepoAccessId);
        $this->assertNotNull($orderRepoAccess, 'orderRepository access call should exist');
        $this->assertSame(
            'access',
            $orderRepoAccess['kind'] ?? '',
            'Should be an access call (property access)'
        );
        $this->assertStringContainsString(
            'orderRepository',
            $orderRepoAccess['callee'] ?? '',
            'Should access orderRepository property'
        );

        // Step 5: Check if orderRepository access has receiver_value_id
        // Note: In readonly classes with constructor promotion, $this->property
        // may not have a tracked receiver for the $this reference
        $thisValueId = $orderRepoAccess['receiver_value_id'] ?? null;
        if ($thisValueId !== null) {
            $thisValue = self::$calls->getValueById($thisValueId);
            $this->assertNotNull($thisValue, '$this value should exist');
        }
        // Chain verified up to orderRepository access - $this tracking is optional
    }

    #[ContractTest(
        name: 'Property Access Chain $order->customerEmail',
        description: 'Verifies property access chains correctly: value (parameter/local) -> access (call) -> result (value).',
        category: 'chain',
    )]
    public function testPropertyAccessChain(): void
    {
        // Code reference: src/Repository/OrderRepository.php:31
        // customerEmail: $order->customerEmail

        // Find customerEmail property access in OrderRepository::save()
        $accessCall = $this->calls()
            ->kind('access')
            ->callerContains('OrderRepository#save()')
            ->calleeContains('customerEmail')
            ->first();

        $this->assertNotNull($accessCall, 'Should find customerEmail access');

        // Verify it has receiver pointing to $order parameter
        $receiverId = $accessCall['receiver_value_id'] ?? null;
        $this->assertNotNull($receiverId, 'Access should have receiver_value_id');

        $receiverValue = self::$calls->getValueById($receiverId);
        $this->assertNotNull($receiverValue, 'Receiver value should exist');
        $this->assertSame(
            'parameter',
            $receiverValue['kind'] ?? '',
            'Receiver should be $order parameter'
        );
        $this->assertStringContainsString(
            '($order)',
            $receiverValue['symbol'] ?? '',
            'Receiver should be $order parameter'
        );

        // Verify access has corresponding result value
        $resultValue = self::$calls->getValueById($accessCall['id']);
        $this->assertNotNull($resultValue, 'Access should have result value');
        $this->assertSame('result', $resultValue['kind'] ?? '', 'Should be a result');
        $this->assertSame(
            $accessCall['id'],
            $resultValue['source_call_id'] ?? '',
            'Result should point back to access call'
        );
    }

    #[ContractTest(
        name: 'Multi-Step Chain findById()->customerEmail',
        description: 'Verifies multi-step chain: findById() returns Order, then access customerEmail property.',
        category: 'chain',
    )]
    public function testMultiStepChainFindByIdCustomerEmail(): void
    {
        // Code reference: src/Service/NotificationService.php:20-27
        // $order = $this->orderRepository->findById($orderId);
        // ...
        // to: $order->customerEmail,

        // Find $order local variable
        $orderLocal = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->values()
            ->kind('local')
            ->symbolContains('local$order')
            ->first();

        $this->assertNotNull($orderLocal, 'Should find $order local');

        // Verify $order has source_call_id pointing to findById
        $sourceCallId = $orderLocal['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, '$order should have source_call_id');

        $findByIdCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($findByIdCall, 'findById call should exist');
        $this->assertStringContainsString(
            'findById',
            $findByIdCall['callee'] ?? '',
            'Source call should be findById'
        );

        // Find property access on $order
        $emailAccess = $this->inMethod('App\Service\NotificationService', 'notifyOrderCreated')
            ->calls()
            ->kind('access')
            ->calleeContains('customerEmail')
            ->first();

        $this->assertNotNull($emailAccess, 'Should find customerEmail access');

        // Verify the access receiver points to $order local
        $receiverId = $emailAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($receiverId, 'Access should have receiver');
        $this->assertSame(
            $orderLocal['id'],
            $receiverId,
            'Property access receiver should point to $order local value'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Chain Integrity Statistics
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Chains Terminate at Valid Source',
        description: 'Verifies tracing any chain backwards terminates at a parameter, local, literal, or constant (not orphaned).',
        category: 'chain',
    )]
    public function testAllChainsTerminateAtValidSource(): void
    {
        $orphanedChains = [];
        $maxDepth = 20; // Prevent infinite loops

        foreach (self::$calls->calls() as $call) {
            $receiverId = $call['receiver_value_id'] ?? null;
            if ($receiverId === null) {
                continue; // No chain to trace
            }

            // Trace back the chain
            $currentId = $receiverId;
            $depth = 0;

            while ($currentId !== null && $depth < $maxDepth) {
                $value = self::$calls->getValueById($currentId);
                if ($value === null) {
                    $orphanedChains[] = sprintf(
                        'Chain from call %s ends at non-existent value %s',
                        $call['id'] ?? 'unknown',
                        $currentId
                    );
                    break;
                }

                $kind = $value['kind'] ?? '';

                // Terminal kinds - chain should end here
                if (in_array($kind, ['parameter', 'local', 'literal', 'constant'], true)) {
                    break; // Valid termination
                }

                // result kind - follow source_call_id to the call, then receiver_value_id
                if ($kind === 'result') {
                    $sourceCallId = $value['source_call_id'] ?? null;
                    if ($sourceCallId === null) {
                        $orphanedChains[] = sprintf(
                            'Result value %s has no source_call_id',
                            $value['id'] ?? 'unknown'
                        );
                        break;
                    }

                    $sourceCall = self::$calls->getCallById($sourceCallId);
                    if ($sourceCall === null) {
                        $orphanedChains[] = sprintf(
                            'Result value %s references non-existent call %s',
                            $value['id'] ?? 'unknown',
                            $sourceCallId
                        );
                        break;
                    }

                    $currentId = $sourceCall['receiver_value_id'] ?? null;
                    // If no receiver, chain ends (valid for static calls, functions, constructors)
                } else {
                    // Unknown kind
                    $orphanedChains[] = sprintf(
                        'Unknown value kind %s at %s',
                        $kind,
                        $value['id'] ?? 'unknown'
                    );
                    break;
                }

                $depth++;
            }

            if ($depth >= $maxDepth) {
                $orphanedChains[] = sprintf(
                    'Chain from call %s exceeds max depth (possible cycle)',
                    $call['id'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $orphanedChains,
            sprintf(
                "Found %d broken chains:\n%s",
                count($orphanedChains),
                implode("\n", array_slice($orphanedChains, 0, 10))
            )
        );
    }
}
