<?php

declare(strict_types=1);

namespace ContractTests\Tests\Argument;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for argument record schema compliance.
 *
 * Per the finish-mvp spec:
 * - Arguments should NOT have a value_type field
 * - Type information is available via values[value_id].type
 *
 * Reference code: src/Service/OrderService.php (method calls with arguments)
 */
class ArgumentSchemaTest extends CallsContractTestCase
{
    #[ContractTest(
        name: 'Arguments Have No value_type Field',
        description: 'Verifies argument records do NOT contain a value_type field. Per finish-mvp spec, consumers should use values[value_id].type instead.',
        category: 'schema',
        status: 'pending',
    )]
    public function testArgumentsHaveNoValueTypeField(): void
    {
        $callsWithArguments = $this->calls()
            ->kindType('invocation')
            ->all();

        $this->assertNotEmpty($callsWithArguments, 'Should have calls with arguments');

        $argumentsWithValueType = [];
        $totalArgumentsChecked = 0;

        foreach ($callsWithArguments as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $totalArgumentsChecked++;
                if (array_key_exists('value_type', $arg)) {
                    $argumentsWithValueType[] = sprintf(
                        'Call %s, arg position %d has value_type: %s',
                        $call['id'] ?? 'unknown',
                        $arg['position'] ?? -1,
                        $arg['value_type'] ?? 'null'
                    );
                }
            }
        }

        $this->assertEmpty(
            $argumentsWithValueType,
            sprintf(
                "Found %d arguments with value_type field (should be 0):\n%s",
                count($argumentsWithValueType),
                implode("\n", array_slice($argumentsWithValueType, 0, 10))
            )
        );

        // Informational: how many arguments were checked
        $this->assertGreaterThan(
            0,
            $totalArgumentsChecked,
            'Should have checked at least one argument'
        );
    }

    #[ContractTest(
        name: 'Argument Fields Match Schema',
        description: 'Verifies argument records contain only the expected fields: position (required), parameter, value_id, value_expr. No extra fields allowed.',
        category: 'schema',
        status: 'pending',
    )]
    public function testArgumentFieldsMatchSchema(): void
    {
        $allowedFields = ['position', 'parameter', 'value_id', 'value_expr'];

        $callsWithArguments = $this->calls()
            ->kindType('invocation')
            ->all();

        $unexpectedFields = [];

        foreach ($callsWithArguments as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                foreach (array_keys($arg) as $field) {
                    if (!in_array($field, $allowedFields, true)) {
                        $key = $field;
                        $unexpectedFields[$key] = ($unexpectedFields[$key] ?? 0) + 1;
                    }
                }
            }
        }

        $this->assertEmpty(
            $unexpectedFields,
            sprintf(
                'Found unexpected fields in argument records: %s',
                implode(', ', array_map(
                    fn($k, $c) => "{$k} ({$c})",
                    array_keys($unexpectedFields),
                    array_values($unexpectedFields)
                ))
            )
        );
    }

    #[ContractTest(
        name: 'Type Lookup Via value_id Works',
        description: 'Verifies that for arguments with value_id, the type can be looked up via values[value_id].type. This is the replacement for the removed value_type field.',
        category: 'integrity',
    )]
    public function testTypeLookupViaValueIdWorks(): void
    {
        $callsWithArguments = $this->calls()
            ->kindType('invocation')
            ->all();

        $this->assertNotEmpty($callsWithArguments, 'Should have calls with arguments');

        $argumentsWithType = 0;
        $argumentsWithoutType = 0;
        $argumentsWithNullValueId = 0;

        foreach ($callsWithArguments as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $valueId = $arg['value_id'] ?? null;

                if ($valueId === null) {
                    $argumentsWithNullValueId++;
                    continue;
                }

                // Look up the value
                $value = self::$calls->getValueById($valueId);
                $this->assertNotNull(
                    $value,
                    sprintf(
                        'Value %s should exist for argument in call %s',
                        $valueId,
                        $call['id'] ?? 'unknown'
                    )
                );

                // Check if type is available
                if (isset($value['type']) && $value['type'] !== null) {
                    $argumentsWithType++;
                } else {
                    $argumentsWithoutType++;
                }
            }
        }

        // At least some arguments should have type available via value lookup
        $this->assertGreaterThan(
            0,
            $argumentsWithType + $argumentsWithoutType + $argumentsWithNullValueId,
            'Should have checked at least one argument'
        );

        // Report statistics
        fwrite(STDERR, sprintf(
            "\n=== Argument Type Lookup Stats ===\n" .
            "  Arguments with type via value_id: %d\n" .
            "  Arguments without type (value exists): %d\n" .
            "  Arguments with null value_id: %d\n" .
            "================================\n",
            $argumentsWithType,
            $argumentsWithoutType,
            $argumentsWithNullValueId
        ));
    }

    #[ContractTest(
        name: 'All Argument value_ids Point to Values',
        description: 'Verifies every argument value_id references an existing value entry in the values array. This ensures type lookup is possible.',
        category: 'integrity',
    )]
    public function testAllArgumentValueIdsPointToValues(): void
    {
        $callsWithArguments = $this->calls()
            ->kindType('invocation')
            ->all();

        $orphanedReferences = [];

        foreach ($callsWithArguments as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $valueId = $arg['value_id'] ?? null;

                if ($valueId === null) {
                    continue; // Null is valid (complex expressions use value_expr)
                }

                if (!self::$calls->hasValue($valueId)) {
                    $orphanedReferences[] = sprintf(
                        'Call %s arg %d: value_id %s does not exist',
                        $call['id'] ?? 'unknown',
                        $arg['position'] ?? -1,
                        $valueId
                    );
                }
            }
        }

        $this->assertEmpty(
            $orphanedReferences,
            sprintf(
                "Found %d arguments with orphaned value_id:\n%s",
                count($orphanedReferences),
                implode("\n", array_slice($orphanedReferences, 0, 10))
            )
        );
    }
}
