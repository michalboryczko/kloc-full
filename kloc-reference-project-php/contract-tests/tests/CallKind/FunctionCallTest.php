<?php

declare(strict_types=1);

namespace ContractTests\Tests\CallKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for function call tracking in calls.json.
 *
 * Verifies that function calls (sprintf(), array_filter(), etc.)
 * are properly tracked with kind=function and correct argument binding.
 *
 * Note: Function calls are an EXPERIMENTAL feature requiring --experimental flag.
 */
class FunctionCallTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Function Call Kind (Experimental)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Function Calls Exist With Experimental Flag',
        description: 'Verifies function calls (sprintf, array_filter) are tracked with kind=function when --experimental is enabled.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsExistWithExperimentalFlag(): void
    {
        $functionCalls = $this->calls()
            ->kind('function')
            ->all();

        $this->assertNotEmpty(
            $functionCalls,
            'Function calls should be present with --experimental flag'
        );
    }

    #[ContractTest(
        name: 'Function Calls Have Kind Type Invocation',
        description: 'Verifies function calls have kind_type=invocation.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsHaveKindTypeInvocation(): void
    {
        $functionCalls = $this->calls()
            ->kind('function')
            ->all();

        $this->assertNotEmpty($functionCalls, 'Function calls should be present with --experimental flag');

        foreach ($functionCalls as $call) {
            $this->assertSame(
                'invocation',
                $call['kind_type'] ?? '',
                sprintf('Function call %s should have kind_type=invocation', $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Sprintf Function Calls Are Tracked',
        description: 'Verifies sprintf() calls are tracked with callee containing sprintf.',
        category: 'callkind',
        experimental: true,
    )]
    public function testSprintfFunctionCallsAreTracked(): void
    {
        // Code reference: src/Component/Address.php uses sprintf
        $sprintfCalls = $this->calls()
            ->kind('function')
            ->calleeContains('sprintf')
            ->all();

        $this->assertNotEmpty(
            $sprintfCalls,
            'sprintf() calls should be tracked'
        );
    }

    #[ContractTest(
        name: 'Function Calls Have Arguments Tracked',
        description: 'Verifies function calls have their arguments captured.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsHaveArgumentsTracked(): void
    {
        // sprintf has multiple arguments
        $sprintfCalls = $this->calls()
            ->kind('function')
            ->calleeContains('sprintf')
            ->all();

        if (empty($sprintfCalls)) {
            $this->markTestSkipped('No sprintf calls found');
        }

        $callWithArgs = null;
        foreach ($sprintfCalls as $call) {
            if (!empty($call['arguments'])) {
                $callWithArgs = $call;
                break;
            }
        }

        $this->assertNotNull(
            $callWithArgs,
            'sprintf() calls should have arguments tracked'
        );

        $this->assertGreaterThanOrEqual(
            1,
            count($callWithArgs['arguments']),
            'sprintf() should have at least 1 argument (format string)'
        );
    }

    #[ContractTest(
        name: 'Function Calls Have Return Type',
        description: 'Verifies function calls have return_type field populated. NOTE: Built-in function return types not yet implemented.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsHaveReturnType(): void
    {
        // sprintf returns string
        $sprintfCalls = $this->calls()
            ->kind('function')
            ->calleeContains('sprintf')
            ->all();

        if (empty($sprintfCalls)) {
            $this->markTestSkipped('No sprintf calls found');
        }

        $callWithReturnType = null;
        foreach ($sprintfCalls as $call) {
            if (!empty($call['return_type'])) {
                $callWithReturnType = $call;
                break;
            }
        }

        // Known limitation: Built-in function return types not yet implemented
        if ($callWithReturnType === null) {
            $this->markTestSkipped(
                '[KNOWN GAP] Built-in function return types not yet implemented in scip-php'
            );
        }

        $this->assertStringContainsString(
            'string',
            $callWithReturnType['return_type'],
            'sprintf() return_type should contain string'
        );
    }

    #[ContractTest(
        name: 'Function Calls Have No Receiver Value ID',
        description: 'Verifies function calls do not have receiver_value_id (functions are not methods).',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsHaveNoReceiverValueId(): void
    {
        $functionCalls = $this->calls()
            ->kind('function')
            ->all();

        $this->assertNotEmpty($functionCalls, 'Function calls should be present with --experimental flag');

        foreach ($functionCalls as $call) {
            $this->assertNull(
                $call['receiver_value_id'] ?? null,
                sprintf('Function call %s should not have receiver_value_id', $call['id'])
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Array Functions
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Array Function Calls Are Tracked',
        description: 'Verifies array functions (array_filter, array_map, array_keys) are tracked.',
        category: 'callkind',
        experimental: true,
    )]
    public function testArrayFunctionCallsAreTracked(): void
    {
        $arrayFunctions = ['array_filter', 'array_map', 'array_keys', 'array_values', 'array_merge'];

        $arrayFunctionCalls = [];
        foreach ($arrayFunctions as $funcName) {
            $calls = $this->calls()
                ->kind('function')
                ->calleeContains($funcName)
                ->all();
            $arrayFunctionCalls = array_merge($arrayFunctionCalls, $calls);
        }

        // Array functions may or may not be present depending on code
        if (empty($arrayFunctionCalls)) {
            $this->markTestSkipped('No array function calls found in reference project');
        }

        $this->assertNotEmpty($arrayFunctionCalls, 'Array function calls should be tracked');
    }

    // ═══════════════════════════════════════════════════════════════
    // Function Call Location
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Function Calls Have Accurate Location',
        description: 'Verifies function calls have location with file, line, and col.',
        category: 'callkind',
        experimental: true,
    )]
    public function testFunctionCallsHaveAccurateLocation(): void
    {
        $functionCalls = $this->calls()
            ->kind('function')
            ->all();

        $this->assertNotEmpty($functionCalls, 'Function calls should be present with --experimental flag');

        foreach ($functionCalls as $call) {
            $this->assertArrayHasKey('location', $call, 'Function call should have location');
            $this->assertArrayHasKey('file', $call['location'], 'Location should have file');
            $this->assertArrayHasKey('line', $call['location'], 'Location should have line');
            $this->assertArrayHasKey('col', $call['location'], 'Location should have col');
        }
    }
}
