<?php

declare(strict_types=1);

namespace ContractTests\Tests\Chain;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for nested object property access chains.
 *
 * Verifies that nested chains like $customer->contact->email correctly track:
 * - Shared receivers when accessing multiple properties on same nested object
 * - Value flow from entity properties through service to response DTOs
 * - Method calls on nested object results
 */
class NestedChainTest extends CallsContractTestCase
{
    // ================================================================
    // Scenario 1: Nested Property Access Chains Share Receiver
    // ================================================================

    #[ContractTest(
        name: 'Nested Contact Property Accesses Share Receiver',
        description: 'Verifies $customer->contact->email and $customer->contact->phone both have the same receiver_value_id for the contact access (pointing to the result of $customer->contact).',
        category: 'chain',
    )]
    public function testNestedContactPropertyAccessesShareReceiver(): void
    {
        // Code reference: src/Service/CustomerService.php:40-41
        // $email = $customer->contact->email;
        // $phone = $customer->contact->phone;

        // Find the email and phone property accesses within getCustomerById
        $emailAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$email')
            ->first();

        $phoneAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$phone')
            ->first();

        // Both should exist
        $this->assertNotNull($emailAccess, 'Should find email property access');
        $this->assertNotNull($phoneAccess, 'Should find phone property access');

        // Both should have receiver_value_id pointing to contact result
        $emailReceiverId = $emailAccess['receiver_value_id'] ?? null;
        $phoneReceiverId = $phoneAccess['receiver_value_id'] ?? null;

        $this->assertNotNull($emailReceiverId, 'email access should have receiver_value_id');
        $this->assertNotNull($phoneReceiverId, 'phone access should have receiver_value_id');

        // The receivers should point to the result of contact accesses
        $emailReceiverValue = self::$calls->getValueById($emailReceiverId);
        $phoneReceiverValue = self::$calls->getValueById($phoneReceiverId);

        $this->assertNotNull($emailReceiverValue, 'email receiver value should exist');
        $this->assertNotNull($phoneReceiverValue, 'phone receiver value should exist');

        // Both should be result kind (from contact access)
        $this->assertSame('result', $emailReceiverValue['kind'] ?? '', 'email receiver should be result');
        $this->assertSame('result', $phoneReceiverValue['kind'] ?? '', 'phone receiver should be result');
    }

    #[ContractTest(
        name: 'Nested Address Property Accesses Share Receiver',
        description: 'Verifies $customer->address->street and $customer->address->city both have the same receiver_value_id for the address access.',
        category: 'chain',
    )]
    public function testNestedAddressPropertyAccessesShareReceiver(): void
    {
        // Code reference: src/Service/CustomerService.php:45-46
        // $street = $customer->address->street;
        // $city = $customer->address->city;

        // Find the street and city property accesses
        $streetAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$street')
            ->first();

        $cityAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$city')
            ->first();

        // Both should exist
        $this->assertNotNull($streetAccess, 'Should find street property access');
        $this->assertNotNull($cityAccess, 'Should find city property access');

        // Both should have receiver_value_id
        $streetReceiverId = $streetAccess['receiver_value_id'] ?? null;
        $cityReceiverId = $cityAccess['receiver_value_id'] ?? null;

        $this->assertNotNull($streetReceiverId, 'street access should have receiver_value_id');
        $this->assertNotNull($cityReceiverId, 'city access should have receiver_value_id');

        // The receivers should point to the result of address accesses
        $streetReceiverValue = self::$calls->getValueById($streetReceiverId);
        $cityReceiverValue = self::$calls->getValueById($cityReceiverId);

        $this->assertNotNull($streetReceiverValue, 'street receiver value should exist');
        $this->assertNotNull($cityReceiverValue, 'city receiver value should exist');

        // Both should be result kind (from address access)
        $this->assertSame('result', $streetReceiverValue['kind'] ?? '', 'street receiver should be result');
        $this->assertSame('result', $cityReceiverValue['kind'] ?? '', 'city receiver should be result');
    }

    // ================================================================
    // Scenario 2: Multiple Nested Objects Share Parent Receiver
    // ================================================================

    #[ContractTest(
        name: 'Contact and Address Accesses Share Customer Receiver',
        description: 'Verifies that $customer->contact and $customer->address property accesses both have the same receiver_value_id (the $customer local variable).',
        category: 'chain',
    )]
    public function testContactAndAddressAccessesShareCustomerReceiver(): void
    {
        // Code reference: src/Service/CustomerService.php:40,45
        // $email = $customer->contact->email;  // contact access
        // $street = $customer->address->street; // address access

        // Find the contact property accesses (there may be multiple)
        $contactAccesses = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$contact')
            ->all();

        // Find the address property accesses (there may be multiple)
        $addressAccesses = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$address')
            ->all();

        $this->assertNotEmpty($contactAccesses, 'Should find contact property accesses');
        $this->assertNotEmpty($addressAccesses, 'Should find address property accesses');

        // Get receiver IDs from all contact and address accesses
        $contactReceiverIds = array_filter(array_map(
            fn($c) => $c['receiver_value_id'] ?? null,
            $contactAccesses
        ));
        $addressReceiverIds = array_filter(array_map(
            fn($c) => $c['receiver_value_id'] ?? null,
            $addressAccesses
        ));

        $this->assertNotEmpty($contactReceiverIds, 'contact accesses should have receiver_value_ids');
        $this->assertNotEmpty($addressReceiverIds, 'address accesses should have receiver_value_ids');

        // All should point to the same $customer value
        $allReceiverIds = array_unique(array_merge($contactReceiverIds, $addressReceiverIds));

        // The receivers should all point to the $customer local
        foreach ($allReceiverIds as $receiverId) {
            $receiverValue = self::$calls->getValueById($receiverId);
            $this->assertNotNull($receiverValue, "Receiver value $receiverId should exist");

            // Should be local or parameter (depending on how $customer is declared)
            $kind = $receiverValue['kind'] ?? '';
            $this->assertTrue(
                in_array($kind, ['local', 'parameter', 'result'], true),
                "Receiver should be local, parameter, or result, got: $kind"
            );
        }
    }

    // ================================================================
    // Scenario 3: Value Flow Traceability (Full Chain Walk)
    // ================================================================

    #[ContractTest(
        name: 'Value Flow from Entity to Service DTO',
        description: 'Verifies we can trace the value of $street from CustomerOutput constructor argument back through the chain: argument -> $street local -> address->street access -> address access -> $customer.',
        category: 'chain',
    )]
    public function testValueFlowFromEntityToResponse(): void
    {
        // Code reference: src/Service/CustomerService.php:55,65
        // $street = $customer->address->street;
        // return new CustomerOutput(..., street: $street, ...);

        // Find the $street local variable in getCustomerById
        $streetLocal = $this->inMethod('App\Service\CustomerService', 'getCustomerById')
            ->values()
            ->kind('local')
            ->symbolContains('local$street')
            ->first();

        $this->assertNotNull($streetLocal, 'Should find $street local variable');

        // Verify $street has source_call_id pointing to the address->street access
        $sourceCallId = $streetLocal['source_call_id'] ?? null;
        $this->assertNotNull($sourceCallId, '$street should have source_call_id');

        $sourceCall = self::$calls->getCallById($sourceCallId);
        $this->assertNotNull($sourceCall, 'Source call should exist');
        $this->assertSame('access', $sourceCall['kind'] ?? '', 'Source should be access call');
        $this->assertStringContainsString(
            'street',
            $sourceCall['callee'] ?? '',
            'Source call should access street property'
        );

        // Get the receiver of the street access (should be result of address access)
        $streetAccessReceiverId = $sourceCall['receiver_value_id'] ?? null;
        $this->assertNotNull($streetAccessReceiverId, 'street access should have receiver_value_id');

        $streetAccessReceiver = self::$calls->getValueById($streetAccessReceiverId);
        $this->assertNotNull($streetAccessReceiver, 'street access receiver should exist');
        $this->assertSame('result', $streetAccessReceiver['kind'] ?? '', 'street access receiver should be result');

        // Get the source call of the address result (should be address access)
        $addressAccessId = $streetAccessReceiver['source_call_id'] ?? null;
        $this->assertNotNull($addressAccessId, 'address result should have source_call_id');

        $addressAccess = self::$calls->getCallById($addressAccessId);
        $this->assertNotNull($addressAccess, 'address access should exist');
        $this->assertSame('access', $addressAccess['kind'] ?? '', 'Should be access call');
        $this->assertStringContainsString(
            'address',
            $addressAccess['callee'] ?? '',
            'Should access address property'
        );

        // Get the receiver of address access (should be $customer local or parameter)
        $addressReceiverId = $addressAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($addressReceiverId, 'address access should have receiver_value_id');

        $customerValue = self::$calls->getValueById($addressReceiverId);
        $this->assertNotNull($customerValue, 'customer value should exist');

        // $customer should be local or parameter
        $kind = $customerValue['kind'] ?? '';
        $this->assertTrue(
            in_array($kind, ['local', 'parameter'], true),
            "customer value should be local or parameter, got: $kind"
        );
    }

    // ================================================================
    // Scenario 4: Nested Method Call Chains
    // ================================================================

    #[ContractTest(
        name: 'Method Call on Nested Object Result',
        description: 'Verifies $customer->contact->getFormattedEmail() has correct chain: getFormattedEmail() receiver points to contact access result, contact access receiver points to $customer.',
        category: 'chain',
    )]
    public function testMethodCallOnNestedObjectResult(): void
    {
        // Code reference: src/Service/CustomerService.php:67
        // return $customer->contact->getFormattedEmail();

        // Find the getFormattedEmail() method call
        $methodCall = $this->calls()
            ->kind('method')
            ->callerContains('CustomerService#getFormattedCustomerEmail()')
            ->calleeContains('getFormattedEmail')
            ->first();

        $this->assertNotNull($methodCall, 'Should find getFormattedEmail() method call');

        // Verify it has receiver_value_id
        $methodReceiverId = $methodCall['receiver_value_id'] ?? null;
        $this->assertNotNull($methodReceiverId, 'getFormattedEmail() should have receiver_value_id');

        // The receiver should be result of contact access
        $receiverValue = self::$calls->getValueById($methodReceiverId);
        $this->assertNotNull($receiverValue, 'Method receiver value should exist');
        $this->assertSame('result', $receiverValue['kind'] ?? '', 'Method receiver should be result');

        // Get the source call (contact access)
        $contactAccessId = $receiverValue['source_call_id'] ?? null;
        $this->assertNotNull($contactAccessId, 'Contact result should have source_call_id');

        $contactAccess = self::$calls->getCallById($contactAccessId);
        $this->assertNotNull($contactAccess, 'Contact access should exist');
        $this->assertSame('access', $contactAccess['kind'] ?? '', 'Should be access call');
        $this->assertStringContainsString(
            'contact',
            $contactAccess['callee'] ?? '',
            'Should access contact property'
        );

        // Verify contact access has receiver pointing to $customer
        $contactReceiverId = $contactAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($contactReceiverId, 'contact access should have receiver_value_id');

        $customerValue = self::$calls->getValueById($contactReceiverId);
        $this->assertNotNull($customerValue, 'customer value should exist');

        // $customer is a parameter in getFormattedCustomerEmail
        $this->assertSame('parameter', $customerValue['kind'] ?? '', 'customer should be parameter');
    }

    #[ContractTest(
        name: 'Method Call on Address Nested Object',
        description: 'Verifies $customer->address->getFullAddress() has correct chain structure.',
        category: 'chain',
    )]
    public function testMethodCallOnAddressNestedObject(): void
    {
        // Code reference: src/Service/CustomerService.php:80
        // return $customer->address->getFullAddress();

        // Find the getFullAddress() method call
        $methodCall = $this->calls()
            ->kind('method')
            ->callerContains('CustomerService#getCustomerFullAddress()')
            ->calleeContains('getFullAddress')
            ->first();

        $this->assertNotNull($methodCall, 'Should find getFullAddress() method call');

        // Verify it has receiver_value_id pointing to address result
        $methodReceiverId = $methodCall['receiver_value_id'] ?? null;
        $this->assertNotNull($methodReceiverId, 'getFullAddress() should have receiver_value_id');

        $receiverValue = self::$calls->getValueById($methodReceiverId);
        $this->assertNotNull($receiverValue, 'Method receiver value should exist');
        $this->assertSame('result', $receiverValue['kind'] ?? '', 'Method receiver should be result');

        // Get the source call (address access)
        $addressAccessId = $receiverValue['source_call_id'] ?? null;
        $this->assertNotNull($addressAccessId, 'Address result should have source_call_id');

        $addressAccess = self::$calls->getCallById($addressAccessId);
        $this->assertNotNull($addressAccess, 'Address access should exist');
        $this->assertSame('access', $addressAccess['kind'] ?? '', 'Should be access call');
    }

    // ================================================================
    // Scenario 5: Deep Chain Integrity
    // ================================================================

    #[ContractTest(
        name: 'Deep Chain Walk from Response to Entity',
        description: 'Verifies complete chain integrity: walks the full chain from a nested property access backwards through all receivers to the original source, verifying each link exists.',
        category: 'chain',
    )]
    public function testDeepChainWalkFromResponseToEntity(): void
    {
        // Code reference: src/Service/CustomerService.php
        // Tests that we can walk backwards through the full chain

        // Start from a nested property access
        $emailAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('$email')
            ->first();

        $this->assertNotNull($emailAccess, 'Should find email property access');

        // Walk the chain backwards
        $chainDepth = 0;
        $maxDepth = 10;
        $currentId = $emailAccess['receiver_value_id'] ?? null;
        $chainLog = ['email_access'];

        while ($currentId !== null && $chainDepth < $maxDepth) {
            $value = self::$calls->getValueById($currentId);
            $this->assertNotNull($value, "Chain link $chainDepth should exist: $currentId");

            $kind = $value['kind'] ?? '';
            $chainLog[] = "$kind:$currentId";

            // Terminal kinds - chain should end here
            if (in_array($kind, ['parameter', 'local', 'literal', 'constant'], true)) {
                break;
            }

            // Result kind - follow source_call_id
            if ($kind === 'result') {
                $sourceCallId = $value['source_call_id'] ?? null;
                $this->assertNotNull($sourceCallId, "Result value should have source_call_id");

                $sourceCall = self::$calls->getCallById($sourceCallId);
                $this->assertNotNull($sourceCall, "Source call should exist: $sourceCallId");

                $currentId = $sourceCall['receiver_value_id'] ?? null;
            } else {
                $this->fail("Unexpected value kind in chain: $kind");
            }

            $chainDepth++;
        }

        // Verify chain terminated properly (not at max depth)
        $this->assertLessThan(
            $maxDepth,
            $chainDepth,
            'Chain should terminate before max depth. Chain: ' . implode(' -> ', $chainLog)
        );

        // Verify we reached a terminal value
        $terminalValue = self::$calls->getValueById($currentId);
        if ($terminalValue !== null) {
            $terminalKind = $terminalValue['kind'] ?? '';
            $this->assertTrue(
                in_array($terminalKind, ['parameter', 'local', 'literal', 'constant'], true),
                "Chain should terminate at parameter/local/literal/constant, got: $terminalKind"
            );
        }
    }

    #[ContractTest(
        name: 'No Orphaned References in Nested Chains',
        description: 'Verifies that all receiver_value_id and source_call_id references in CustomerService nested chains point to existing entries.',
        category: 'chain',
    )]
    public function testNoOrphanedReferencesInNestedChains(): void
    {
        // Get all calls in CustomerService
        $customerServiceCalls = $this->calls()
            ->callerContains('CustomerService')
            ->all();

        $this->assertNotEmpty($customerServiceCalls, 'Should have calls in CustomerService');

        $orphanedReferences = [];

        foreach ($customerServiceCalls as $call) {
            $callId = $call['id'] ?? 'unknown';

            // Check receiver_value_id
            $receiverId = $call['receiver_value_id'] ?? null;
            if ($receiverId !== null) {
                $value = self::$calls->getValueById($receiverId);
                if ($value === null) {
                    $orphanedReferences[] = "Call $callId has orphaned receiver_value_id: $receiverId";
                }
            }

            // Check arguments
            foreach ($call['arguments'] ?? [] as $arg) {
                $argValueId = $arg['value_id'] ?? null;
                if ($argValueId !== null) {
                    $value = self::$calls->getValueById($argValueId);
                    if ($value === null) {
                        $orphanedReferences[] = "Call $callId argument has orphaned value_id: $argValueId";
                    }
                }
            }
        }

        // Check values in CustomerService for orphaned source_call_id
        $customerServiceValues = $this->values()
            ->symbolContains('CustomerService')
            ->all();

        foreach ($customerServiceValues as $value) {
            $valueId = $value['id'] ?? 'unknown';

            // Check source_call_id
            $sourceCallId = $value['source_call_id'] ?? null;
            if ($sourceCallId !== null) {
                $call = self::$calls->getCallById($sourceCallId);
                if ($call === null) {
                    $orphanedReferences[] = "Value $valueId has orphaned source_call_id: $sourceCallId";
                }
            }

            // Check source_value_id
            $sourceValueId = $value['source_value_id'] ?? null;
            if ($sourceValueId !== null) {
                $sourceValue = self::$calls->getValueById($sourceValueId);
                if ($sourceValue === null) {
                    $orphanedReferences[] = "Value $valueId has orphaned source_value_id: $sourceValueId";
                }
            }
        }

        $this->assertEmpty(
            $orphanedReferences,
            sprintf(
                "Found %d orphaned references:\n%s",
                count($orphanedReferences),
                implode("\n", array_slice($orphanedReferences, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Multiple Chains in Same Expression Share Receivers',
        description: 'Verifies that in getCustomerSummary, both $customer->contact->email and $customer->address->city share the same $customer receiver.',
        category: 'chain',
    )]
    public function testMultipleChainsInSameExpressionShareReceivers(): void
    {
        // Code reference: src/Service/CustomerService.php:92-96
        // return sprintf('%s (%s) - %s', $customer->name, $customer->contact->email, $customer->address->city);

        // Find contact and address accesses in getCustomerSummary
        $contactAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerSummary()')
            ->calleeContains('$contact')
            ->first();

        $addressAccess = $this->calls()
            ->kind('access')
            ->callerContains('CustomerService#getCustomerSummary()')
            ->calleeContains('$address')
            ->first();

        $this->assertNotNull($contactAccess, 'Should find contact access in getCustomerSummary');
        $this->assertNotNull($addressAccess, 'Should find address access in getCustomerSummary');

        // Both should have receiver_value_id
        $contactReceiverId = $contactAccess['receiver_value_id'] ?? null;
        $addressReceiverId = $addressAccess['receiver_value_id'] ?? null;

        $this->assertNotNull($contactReceiverId, 'contact access should have receiver_value_id');
        $this->assertNotNull($addressReceiverId, 'address access should have receiver_value_id');

        // Both should point to the same $customer parameter
        $this->assertSame(
            $contactReceiverId,
            $addressReceiverId,
            'contact and address accesses should share the same $customer receiver'
        );

        // Verify the shared receiver is the $customer parameter
        $customerValue = self::$calls->getValueById($contactReceiverId);
        $this->assertNotNull($customerValue, 'customer value should exist');
        $this->assertSame('parameter', $customerValue['kind'] ?? '', 'customer should be parameter');
    }

    // ================================================================
    // Scenario 6: Full Flow from Controller to Entity
    // ================================================================

    #[ContractTest(
        name: 'Full Flow: Controller Response to Entity Property',
        description: 'Verifies the complete data flow from CustomerController creating CustomerResponse, through CustomerService returning CustomerOutput, back to the Entity nested properties. Tests that $output->street in Controller traces to CustomerOutput, which traces to $street local in Service, which traces to $customer->address->street.',
        category: 'chain',
    )]
    public function testFullFlowControllerToEntity(): void
    {
        // Flow: Controller (CustomerResponse) <- Service (CustomerOutput) <- Entity
        //
        // CustomerController::get():
        //   $output = $this->customerService->getCustomerById($id);
        //   return new CustomerResponse(..., street: $output->street, ...);
        //
        // CustomerService::getCustomerById():
        //   $street = $customer->address->street;
        //   return new CustomerOutput(..., street: $street, ...);

        // Step 1: Find the CustomerResponse constructor call in Controller
        $responseConstructor = $this->calls()
            ->kind('constructor')
            ->callerContains('CustomerController#get()')
            ->calleeContains('CustomerResponse')
            ->first();

        $this->assertNotNull($responseConstructor, 'Should find CustomerResponse constructor in Controller');

        // Step 2: Verify the constructor has arguments
        $arguments = $responseConstructor['arguments'] ?? [];
        $this->assertNotEmpty($arguments, 'CustomerResponse constructor should have arguments');

        // Step 3: Find the street argument (should reference $output->street access result)
        $streetArg = null;
        foreach ($arguments as $arg) {
            $paramSymbol = $arg['parameter'] ?? '';
            if (str_contains($paramSymbol, 'street')) {
                $streetArg = $arg;
                break;
            }
        }

        $this->assertNotNull($streetArg, 'Should find street argument in CustomerResponse constructor');

        // Step 4: The street argument value should point to $output->street access result
        $streetArgValueId = $streetArg['value_id'] ?? null;
        $this->assertNotNull($streetArgValueId, 'street argument should have value_id');

        $streetArgValue = self::$calls->getValueById($streetArgValueId);
        $this->assertNotNull($streetArgValue, 'street argument value should exist');

        // Step 5: Verify this is a result from property access on $output
        $this->assertSame('result', $streetArgValue['kind'] ?? '', 'street arg should be result of access');

        $streetAccessId = $streetArgValue['source_call_id'] ?? null;
        $this->assertNotNull($streetAccessId, 'street result should have source_call_id');

        $streetAccess = self::$calls->getCallById($streetAccessId);
        $this->assertNotNull($streetAccess, 'street access call should exist');
        $this->assertSame('access', $streetAccess['kind'] ?? '', 'Should be access call');
        $this->assertStringContainsString('street', $streetAccess['callee'] ?? '', 'Should access street property');

        // Step 6: The receiver should be $output local variable
        $outputReceiverId = $streetAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($outputReceiverId, 'street access should have receiver');

        $outputValue = self::$calls->getValueById($outputReceiverId);
        $this->assertNotNull($outputValue, 'output value should exist');
        $this->assertSame('local', $outputValue['kind'] ?? '', 'output should be local variable');
        $this->assertStringContainsString('output', $outputValue['symbol'] ?? '', 'Should be $output local');
    }

    #[ContractTest(
        name: 'Full Flow: Service Output to Entity Nested Chain',
        description: 'Verifies that CustomerOutput properties in Service can be traced back through nested entity access chains: $output.street <- $street local <- $customer->address->street <- $customer local <- repository call.',
        category: 'chain',
    )]
    public function testFullFlowServiceOutputToEntityChain(): void
    {
        // Find CustomerOutput constructor in getCustomerById
        $outputConstructor = $this->calls()
            ->kind('constructor')
            ->callerContains('CustomerService#getCustomerById()')
            ->calleeContains('CustomerOutput')
            ->first();

        $this->assertNotNull($outputConstructor, 'Should find CustomerOutput constructor in Service');

        // Find the street argument
        $arguments = $outputConstructor['arguments'] ?? [];
        $streetArg = null;
        foreach ($arguments as $arg) {
            $paramSymbol = $arg['parameter'] ?? '';
            if (str_contains($paramSymbol, 'street')) {
                $streetArg = $arg;
                break;
            }
        }

        $this->assertNotNull($streetArg, 'Should find street argument in CustomerOutput constructor');

        // The street argument should reference the $street local variable
        $streetLocalId = $streetArg['value_id'] ?? null;
        $this->assertNotNull($streetLocalId, 'street argument should have value_id');

        $streetLocal = self::$calls->getValueById($streetLocalId);
        $this->assertNotNull($streetLocal, 'street local should exist');
        $this->assertSame('local', $streetLocal['kind'] ?? '', 'should be local variable');

        // $street local should have source_call_id pointing to address->street access
        $streetAccessId = $streetLocal['source_call_id'] ?? null;
        $this->assertNotNull($streetAccessId, '$street should have source_call_id');

        $streetAccess = self::$calls->getCallById($streetAccessId);
        $this->assertNotNull($streetAccess, 'street access should exist');
        $this->assertSame('access', $streetAccess['kind'] ?? '', 'Should be access');
        $this->assertStringContainsString('street', $streetAccess['callee'] ?? '', 'Should access street');

        // street access receiver should be result of address access
        $addressResultId = $streetAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($addressResultId, 'street access should have receiver');

        $addressResult = self::$calls->getValueById($addressResultId);
        $this->assertNotNull($addressResult, 'address result should exist');
        $this->assertSame('result', $addressResult['kind'] ?? '', 'Should be result');

        // address result should have source_call_id pointing to address access
        $addressAccessId = $addressResult['source_call_id'] ?? null;
        $this->assertNotNull($addressAccessId, 'address result should have source_call_id');

        $addressAccess = self::$calls->getCallById($addressAccessId);
        $this->assertNotNull($addressAccess, 'address access should exist');
        $this->assertSame('access', $addressAccess['kind'] ?? '', 'Should be access');
        $this->assertStringContainsString('address', $addressAccess['callee'] ?? '', 'Should access address');

        // address access receiver should be $customer local
        $customerLocalId = $addressAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($customerLocalId, 'address access should have receiver');

        $customerLocal = self::$calls->getValueById($customerLocalId);
        $this->assertNotNull($customerLocal, 'customer local should exist');
        $this->assertSame('local', $customerLocal['kind'] ?? '', '$customer should be local');
        $this->assertStringContainsString('customer', $customerLocal['symbol'] ?? '', 'Should be $customer');

        // $customer should have source_call_id pointing to repository->findById call
        $findByIdId = $customerLocal['source_call_id'] ?? null;
        $this->assertNotNull($findByIdId, '$customer should have source_call_id from findById');

        $findByIdCall = self::$calls->getCallById($findByIdId);
        $this->assertNotNull($findByIdCall, 'findById call should exist');
        $this->assertSame('method', $findByIdCall['kind'] ?? '', 'Should be method call');
        $this->assertStringContainsString('findById', $findByIdCall['callee'] ?? '', 'Should call findById');
    }

    #[ContractTest(
        name: 'Direct Nested Access in Constructor Arguments',
        description: 'Verifies that getCustomerDetails with direct nested access in constructor (email: $customer->contact->email) correctly tracks the chain.',
        category: 'chain',
    )]
    public function testDirectNestedAccessInConstructorArguments(): void
    {
        // In getCustomerDetails: new CustomerOutput(..., email: $customer->contact->email, ...)
        $outputConstructor = $this->calls()
            ->kind('constructor')
            ->callerContains('CustomerService#getCustomerDetails()')
            ->calleeContains('CustomerOutput')
            ->first();

        $this->assertNotNull($outputConstructor, 'Should find CustomerOutput constructor in getCustomerDetails');

        // Find the email argument - it should be a result from nested access chain
        $arguments = $outputConstructor['arguments'] ?? [];
        $emailArg = null;
        foreach ($arguments as $arg) {
            $paramSymbol = $arg['parameter'] ?? '';
            if (str_contains($paramSymbol, 'email')) {
                $emailArg = $arg;
                break;
            }
        }

        $this->assertNotNull($emailArg, 'Should find email argument');

        // The email argument value should be a result (from $customer->contact->email chain)
        $emailValueId = $emailArg['value_id'] ?? null;
        $this->assertNotNull($emailValueId, 'email argument should have value_id');

        $emailValue = self::$calls->getValueById($emailValueId);
        $this->assertNotNull($emailValue, 'email value should exist');
        $this->assertSame('result', $emailValue['kind'] ?? '', 'email should be result from access chain');

        // Trace back through the chain
        $emailAccessId = $emailValue['source_call_id'] ?? null;
        $this->assertNotNull($emailAccessId, 'email result should have source_call_id');

        $emailAccess = self::$calls->getCallById($emailAccessId);
        $this->assertNotNull($emailAccess, 'email access should exist');
        $this->assertStringContainsString('email', $emailAccess['callee'] ?? '', 'Should access email');

        // email access receiver should be result of contact access
        $contactResultId = $emailAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($contactResultId, 'email access should have receiver');

        $contactResult = self::$calls->getValueById($contactResultId);
        $this->assertNotNull($contactResult, 'contact result should exist');
        $this->assertSame('result', $contactResult['kind'] ?? '', 'Should be result from contact access');

        // contact result should trace back to $customer
        $contactAccessId = $contactResult['source_call_id'] ?? null;
        $this->assertNotNull($contactAccessId, 'contact result should have source_call_id');

        $contactAccess = self::$calls->getCallById($contactAccessId);
        $this->assertNotNull($contactAccess, 'contact access should exist');

        $customerReceiverId = $contactAccess['receiver_value_id'] ?? null;
        $this->assertNotNull($customerReceiverId, 'contact access should have receiver');

        $customerValue = self::$calls->getValueById($customerReceiverId);
        $this->assertNotNull($customerValue, 'customer value should exist');
        $this->assertSame('local', $customerValue['kind'] ?? '', '$customer should be local');
    }
}
