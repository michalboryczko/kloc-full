<?php

declare(strict_types=1);

namespace ContractTests\Tests\ValueKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for value kind coverage.
 *
 * Verifies that each value kind is properly tracked in the index
 * and has the correct properties per the schema.
 */
class ValueKindTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Parameter Values
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Parameter Values Have Symbol',
        description: 'Verifies all parameter values have a symbol field. Per schema: parameter kind has symbol, no source_call_id.',
        category: 'valuekind',
    )]
    public function testParameterValuesHaveSymbol(): void
    {
        $parameters = $this->values()
            ->kind('parameter')
            ->all();

        $this->assertNotEmpty($parameters, 'Should find parameter values');

        $missingSymbol = [];
        foreach ($parameters as $param) {
            if (!isset($param['symbol']) || empty($param['symbol'])) {
                $missingSymbol[] = $param['id'] ?? 'unknown';
            }
        }

        $this->assertEmpty(
            $missingSymbol,
            sprintf(
                "Found %d parameter values missing symbol:\n%s",
                count($missingSymbol),
                implode("\n", array_slice($missingSymbol, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Parameter Values No Source Call ID',
        description: 'Verifies parameter values do not have source_call_id. Parameters are inputs, not results of calls.',
        category: 'valuekind',
    )]
    public function testParameterValuesNoSourceCallId(): void
    {
        $parameters = $this->values()
            ->kind('parameter')
            ->all();

        $this->assertNotEmpty($parameters, 'Should find parameter values');

        $hasSourceCallId = [];
        foreach ($parameters as $param) {
            if (isset($param['source_call_id']) && $param['source_call_id'] !== null) {
                $hasSourceCallId[] = sprintf(
                    '%s (symbol: %s)',
                    $param['id'] ?? 'unknown',
                    $param['symbol'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $hasSourceCallId,
            sprintf(
                "Found %d parameter values with source_call_id (unexpected):\n%s",
                count($hasSourceCallId),
                implode("\n", array_slice($hasSourceCallId, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Parameter Symbol Format',
        description: 'Verifies parameter symbols contain ($paramName) pattern. Example: OrderRepository#save().($order)',
        category: 'valuekind',
    )]
    public function testParameterSymbolFormat(): void
    {
        $parameters = $this->values()
            ->kind('parameter')
            ->all();

        $this->assertNotEmpty($parameters, 'Should find parameter values');

        $invalidFormat = [];
        foreach ($parameters as $param) {
            $symbol = $param['symbol'] ?? '';
            // Parameter symbols should contain ($paramName) pattern
            if (!preg_match('/\(\$[a-zA-Z_][a-zA-Z0-9_]*\)/', $symbol)) {
                $invalidFormat[] = sprintf(
                    '%s: %s',
                    $param['id'] ?? 'unknown',
                    $symbol
                );
            }
        }

        $this->assertEmpty(
            $invalidFormat,
            sprintf(
                "Found %d parameter values with invalid symbol format (expected ($name)):\n%s",
                count($invalidFormat),
                implode("\n", array_slice($invalidFormat, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Local Values
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Local Values Have Symbol',
        description: 'Verifies all local values have a symbol field. Per schema: local kind has symbol with @line suffix.',
        category: 'valuekind',
    )]
    public function testLocalValuesHaveSymbol(): void
    {
        $locals = $this->values()
            ->kind('local')
            ->all();

        $this->assertNotEmpty($locals, 'Should find local values');

        $missingSymbol = [];
        foreach ($locals as $local) {
            if (!isset($local['symbol']) || empty($local['symbol'])) {
                $missingSymbol[] = $local['id'] ?? 'unknown';
            }
        }

        $this->assertEmpty(
            $missingSymbol,
            sprintf(
                "Found %d local values missing symbol:\n%s",
                count($missingSymbol),
                implode("\n", array_slice($missingSymbol, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Local Symbol Format with @line',
        description: 'Verifies local symbols contain local$name@line pattern. Example: OrderService#createOrder().local$savedOrder@40',
        category: 'valuekind',
    )]
    public function testLocalSymbolFormatWithLine(): void
    {
        $locals = $this->values()
            ->kind('local')
            ->all();

        $this->assertNotEmpty($locals, 'Should find local values');

        $invalidFormat = [];
        foreach ($locals as $local) {
            $symbol = $local['symbol'] ?? '';
            // Local symbols should contain local$name@line pattern
            if (!preg_match('/local\$[a-zA-Z_][a-zA-Z0-9_]*@\d+/', $symbol)) {
                $invalidFormat[] = sprintf(
                    '%s: %s',
                    $local['id'] ?? 'unknown',
                    $symbol
                );
            }
        }

        $this->assertEmpty(
            $invalidFormat,
            sprintf(
                "Found %d local values with invalid symbol format (expected local\$name@line):\n%s",
                count($invalidFormat),
                implode("\n", array_slice($invalidFormat, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Local Values May Have Source Call ID',
        description: 'Verifies local values assigned from calls have source_call_id. Per schema: local may have source_call_id or source_value_id.',
        category: 'valuekind',
    )]
    public function testLocalValuesMayHaveSourceCallId(): void
    {
        $locals = $this->values()
            ->kind('local')
            ->hasSourceCallId()
            ->all();

        $this->assertNotEmpty(
            $locals,
            'Should find local values with source_call_id. ' .
            'Reference: $savedOrder = $this->orderRepository->save($order)'
        );

        // Verify the source_call_id points to an existing call
        foreach ($locals as $local) {
            $sourceCallId = $local['source_call_id'];
            $this->assertTrue(
                self::$calls->hasCall($sourceCallId),
                sprintf(
                    'Local %s has source_call_id %s that does not exist in calls',
                    $local['id'],
                    $sourceCallId
                )
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Literal Values
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Literal Values Exist',
        description: 'Verifies index contains literal values (strings, integers, etc.). Per schema: literal kind, no symbol.',
        category: 'valuekind',
    )]
    public function testLiteralValuesExist(): void
    {
        $literals = $this->values()
            ->kind('literal')
            ->all();

        $this->assertNotEmpty(
            $literals,
            'Should find literal values. Reference project uses literals like 0, "pending", etc.'
        );
    }

    #[ContractTest(
        name: 'Literal Values No Symbol',
        description: 'Verifies literal values do not have a symbol field. Literals are anonymous values.',
        category: 'valuekind',
    )]
    public function testLiteralValuesNoSymbol(): void
    {
        $literals = $this->values()
            ->kind('literal')
            ->all();

        if (empty($literals)) {
            $this->markTestSkipped('No literal values found in index');
        }

        $hasSymbol = [];
        foreach ($literals as $literal) {
            if (isset($literal['symbol']) && $literal['symbol'] !== null && $literal['symbol'] !== '') {
                $hasSymbol[] = sprintf(
                    '%s: %s',
                    $literal['id'] ?? 'unknown',
                    $literal['symbol']
                );
            }
        }

        $this->assertEmpty(
            $hasSymbol,
            sprintf(
                "Found %d literal values with symbol (unexpected):\n%s",
                count($hasSymbol),
                implode("\n", array_slice($hasSymbol, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Literal Values No Source Call ID',
        description: 'Verifies literal values do not have source_call_id. Literals are not results of calls.',
        category: 'valuekind',
    )]
    public function testLiteralValuesNoSourceCallId(): void
    {
        $literals = $this->values()
            ->kind('literal')
            ->all();

        if (empty($literals)) {
            $this->markTestSkipped('No literal values found in index');
        }

        $hasSourceCallId = [];
        foreach ($literals as $literal) {
            if (isset($literal['source_call_id']) && $literal['source_call_id'] !== null) {
                $hasSourceCallId[] = $literal['id'] ?? 'unknown';
            }
        }

        $this->assertEmpty(
            $hasSourceCallId,
            sprintf(
                "Found %d literal values with source_call_id (unexpected):\n%s",
                count($hasSourceCallId),
                implode("\n", array_slice($hasSourceCallId, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Result Values
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Result Values Exist',
        description: 'Verifies index contains result values (call results). Per schema: result kind, no symbol, always has source_call_id.',
        category: 'valuekind',
    )]
    public function testResultValuesExist(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty(
            $results,
            'Should find result values. Every call that produces a value should have a corresponding result.'
        );
    }

    #[ContractTest(
        name: 'Result Values Have Source Call ID',
        description: 'Verifies all result values have source_call_id. Per schema: result always has source_call_id.',
        category: 'valuekind',
    )]
    public function testResultValuesHaveSourceCallId(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty($results, 'Should find result values');

        $missingSourceCallId = [];
        foreach ($results as $result) {
            if (!isset($result['source_call_id']) || $result['source_call_id'] === null) {
                $missingSourceCallId[] = $result['id'] ?? 'unknown';
            }
        }

        $this->assertEmpty(
            $missingSourceCallId,
            sprintf(
                "Found %d result values missing source_call_id:\n%s",
                count($missingSourceCallId),
                implode("\n", array_slice($missingSourceCallId, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Result Values No Symbol',
        description: 'Verifies result values do not have a symbol field. Results are anonymous intermediate values.',
        category: 'valuekind',
    )]
    public function testResultValuesNoSymbol(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty($results, 'Should find result values');

        $hasSymbol = [];
        foreach ($results as $result) {
            if (isset($result['symbol']) && $result['symbol'] !== null && $result['symbol'] !== '') {
                $hasSymbol[] = sprintf(
                    '%s: %s',
                    $result['id'] ?? 'unknown',
                    $result['symbol']
                );
            }
        }

        $this->assertEmpty(
            $hasSymbol,
            sprintf(
                "Found %d result values with symbol (unexpected):\n%s",
                count($hasSymbol),
                implode("\n", array_slice($hasSymbol, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Result Value ID Matches Source Call ID',
        description: 'Verifies result values have id matching their source_call_id. Per schema: result id equals the call id that produced it.',
        category: 'valuekind',
    )]
    public function testResultValueIdMatchesSourceCallId(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty($results, 'Should find result values');

        $mismatched = [];
        foreach ($results as $result) {
            $id = $result['id'] ?? '';
            $sourceCallId = $result['source_call_id'] ?? '';

            if ($id !== $sourceCallId) {
                $mismatched[] = sprintf(
                    'Result id=%s but source_call_id=%s',
                    $id,
                    $sourceCallId
                );
            }
        }

        $this->assertEmpty(
            $mismatched,
            sprintf(
                "Found %d result values where id != source_call_id:\n%s",
                count($mismatched),
                implode("\n", array_slice($mismatched, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Result Source Call Exists',
        description: 'Verifies every result value source_call_id points to an existing call in the calls array.',
        category: 'valuekind',
    )]
    public function testResultSourceCallExists(): void
    {
        $results = $this->values()
            ->kind('result')
            ->all();

        $this->assertNotEmpty($results, 'Should find result values');

        $orphanedResults = [];
        foreach ($results as $result) {
            $sourceCallId = $result['source_call_id'] ?? '';
            if (!empty($sourceCallId) && !self::$calls->hasCall($sourceCallId)) {
                $orphanedResults[] = sprintf(
                    'Result %s references non-existent call %s',
                    $result['id'] ?? 'unknown',
                    $sourceCallId
                );
            }
        }

        $this->assertEmpty(
            $orphanedResults,
            sprintf(
                "Found %d result values with orphaned source_call_id:\n%s",
                count($orphanedResults),
                implode("\n", array_slice($orphanedResults, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Constant Values
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Constant Values (If Present)',
        description: 'Verifies constant values have symbol, no source_call_id. Per schema: constant kind has symbol.',
        category: 'valuekind',
        status: 'pending',
    )]
    public function testConstantValuesIfPresent(): void
    {
        $constants = $this->values()
            ->kind('constant')
            ->all();

        if (empty($constants)) {
            $this->markTestSkipped(
                'No constant values found in reference project. ' .
                'If added (e.g., class constants), should have symbol, no source_call_id.'
            );
        }

        // Verify properties
        foreach ($constants as $constant) {
            $this->assertNotEmpty(
                $constant['symbol'] ?? '',
                sprintf('Constant %s should have symbol', $constant['id'] ?? 'unknown')
            );
            $this->assertFalse(
                isset($constant['source_call_id']) && $constant['source_call_id'] !== null,
                sprintf('Constant %s should not have source_call_id', $constant['id'] ?? 'unknown')
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Value Type Information
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Parameter Values Have Type',
        description: 'Verifies parameter values have type information when the parameter has a type declaration.',
        category: 'valuekind',
    )]
    public function testParameterValuesHaveType(): void
    {
        // Query parameters that should have types
        $orderParam = $this->values()
            ->kind('parameter')
            ->symbolContains('($order)')
            ->first();

        $this->assertNotNull($orderParam, 'Should find $order parameter');
        $this->assertNotEmpty(
            $orderParam['type'] ?? '',
            '$order parameter should have type (Order)'
        );
        $this->assertStringContainsString(
            'Order',
            $orderParam['type'] ?? '',
            '$order type should contain Order'
        );
    }

    #[ContractTest(
        name: 'Local Values Have Type From Source',
        description: 'Verifies local values inherit type from their source (call result or assigned value).',
        category: 'valuekind',
    )]
    public function testLocalValuesHaveTypeFromSource(): void
    {
        // Query $savedOrder local which is assigned from save() call
        $savedOrderLocal = $this->values()
            ->kind('local')
            ->symbolContains('local$savedOrder')
            ->first();

        $this->assertNotNull($savedOrderLocal, 'Should find $savedOrder local');
        $this->assertNotEmpty(
            $savedOrderLocal['type'] ?? '',
            '$savedOrder should have type'
        );
        $this->assertStringContainsString(
            'Order',
            $savedOrderLocal['type'] ?? '',
            '$savedOrder type should contain Order (return type of save())'
        );
    }

    #[ContractTest(
        name: 'Result Values Have Type From Call Return',
        description: 'Verifies result values have type matching the return_type of their source call.',
        category: 'valuekind',
    )]
    public function testResultValuesHaveTypeFromCallReturn(): void
    {
        // Get a property access result
        $accessCall = $this->calls()
            ->kind('access')
            ->calleeContains('customerEmail')
            ->first();

        $this->assertNotNull($accessCall, 'Should find customerEmail access');

        $resultValue = self::$calls->getValueById($accessCall['id']);
        $this->assertNotNull($resultValue, 'Should find result value for access call');
        $this->assertSame('result', $resultValue['kind'], 'Should be a result value');

        // Result type should match call return_type (if present)
        if (isset($accessCall['return_type'])) {
            $this->assertSame(
                $accessCall['return_type'],
                $resultValue['type'] ?? null,
                'Result value type should match call return_type'
            );
        }
    }
}
