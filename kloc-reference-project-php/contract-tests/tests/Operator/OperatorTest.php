<?php

declare(strict_types=1);

namespace ContractTests\Tests\Operator;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for operator tracking in calls.json.
 *
 * Verifies that PHP operators (coalesce, ternary, match) are tracked
 * with their operand values for data flow analysis.
 */
class OperatorTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Null Coalesce Operator (??)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Null Coalesce Operator Kind Exists',
        description: 'Verifies null coalesce operators ($a ?? $b) are tracked with kind=coalesce, kind_type=operator, left_value_id, right_value_id.',
        category: 'operator',
        experimental: true,
    )]
    public function testNullCoalesceOperatorKindExists(): void
    {
        $coalesceCalls = $this->calls()
            ->kind('coalesce')
            ->all();

        $this->assertNotEmpty(
            $coalesceCalls,
            'Coalesce operators should be present with --experimental flag'
        );

        // Verify structure
        $call = $coalesceCalls[0];
        $this->assertSame('operator', $call['kind_type'] ?? '');
        $this->assertArrayHasKey('left_value_id', $call, 'Coalesce should have left_value_id');
        $this->assertArrayHasKey('right_value_id', $call, 'Coalesce should have right_value_id');
    }

    #[ContractTest(
        name: 'Coalesce Return Type Removes Null',
        description: 'Verifies coalesce return_type correctly removes null from left operand. For ($nullable ?? $default), if left is T|null and right is T, result is T (not T|null).',
        category: 'operator',
        experimental: true,
    )]
    public function testCoalesceReturnTypeRemovesNull(): void
    {
        // Code reference: src/Service/OrderDisplayService.php:45
        // return $order?->status ?? 'unknown';
        // Left type: string|null (from nullsafe property access)
        // Right type: string (literal)
        // Expected result type: string (null removed, since right provides fallback)
        $coalesceCalls = $this->calls()
            ->kind('coalesce')
            ->inFile('OrderDisplayService.php')
            ->all();

        $this->assertNotEmpty($coalesceCalls, 'Coalesce operators should be present with --experimental flag');

        $validReturnTypes = 0;
        $invalidReturnTypes = [];

        foreach ($coalesceCalls as $call) {
            $returnType = $call['return_type'] ?? null;
            if ($returnType === null) {
                continue;
            }

            // Coalesce return_type should NOT contain 'null' since null is handled by fallback
            // Exception: if right operand can also be null, union may contain null
            if (str_contains($returnType, 'null') && str_contains($returnType, 'union')) {
                // This might be valid if the right side can also be null
                // Check if left_value_id's type has null but right doesn't
                $leftId = $call['left_value_id'] ?? null;
                $rightId = $call['right_value_id'] ?? null;

                if ($leftId !== null && $rightId !== null) {
                    $leftValue = self::$calls->getValueById($leftId);
                    $rightValue = self::$calls->getValueById($rightId);

                    $leftType = $leftValue['type'] ?? '';
                    $rightType = $rightValue['type'] ?? '';

                    // If left has null but right doesn't, result shouldn't have null
                    if (str_contains($leftType, 'null') && !str_contains($rightType, 'null')) {
                        $invalidReturnTypes[] = sprintf(
                            '%s: left=%s, right=%s, return=%s (null should be removed)',
                            $call['id'],
                            $leftType,
                            $rightType,
                            $returnType
                        );
                    }
                }
            } else {
                // Return type without null union is correct
                $validReturnTypes++;
            }
        }

        $this->assertEmpty(
            $invalidReturnTypes,
            sprintf(
                "Coalesce return_type incorrectly contains null:\n%s",
                implode("\n", $invalidReturnTypes)
            )
        );

        $this->assertGreaterThan(
            0,
            $validReturnTypes,
            'At least one coalesce should have valid return_type without null'
        );
    }

    #[ContractTest(
        name: 'Coalesce Operands Reference Values',
        description: 'Verifies coalesce left_value_id and right_value_id point to existing values in the values array.',
        category: 'operator',
        experimental: true,
    )]
    public function testCoalesceOperandsReferenceValues(): void
    {
        $coalesceCalls = $this->calls()
            ->kind('coalesce')
            ->all();

        $this->assertNotEmpty($coalesceCalls, 'Coalesce operators should be present with --experimental flag');

        foreach ($coalesceCalls as $call) {
            $leftId = $call['left_value_id'] ?? null;
            $rightId = $call['right_value_id'] ?? null;

            if ($leftId !== null) {
                $this->assertTrue(
                    self::$calls->hasValue($leftId),
                    sprintf('Coalesce %s left_value_id %s should exist', $call['id'], $leftId)
                );
            }

            if ($rightId !== null) {
                $this->assertTrue(
                    self::$calls->hasValue($rightId),
                    sprintf('Coalesce %s right_value_id %s should exist', $call['id'], $rightId)
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Ternary Operator (? :)
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Ternary Operator Kind Exists',
        description: 'Verifies ternary operators ($a ? $b : $c) are tracked with kind=ternary_full, kind_type=operator, and operand IDs.',
        category: 'operator',
        experimental: true,
    )]
    public function testTernaryOperatorKindExists(): void
    {
        $ternaryFullCalls = $this->calls()
            ->kind('ternary_full')
            ->all();

        $ternaryCalls = $this->calls()
            ->kind('ternary')
            ->all();

        $allTernary = array_merge($ternaryFullCalls, $ternaryCalls);

        $this->assertNotEmpty(
            $allTernary,
            'Ternary operators should be present with --experimental flag'
        );

        // Verify structure of first ternary
        $call = $allTernary[0];
        $this->assertSame('operator', $call['kind_type'] ?? '');
    }

    #[ContractTest(
        name: 'Full Ternary Has All Operand IDs',
        description: 'Verifies full ternary ($a ? $b : $c) has condition_value_id, true_value_id, and false_value_id.',
        category: 'operator',
        experimental: true,
    )]
    public function testFullTernaryHasAllOperandIds(): void
    {
        $ternaryFullCalls = $this->calls()
            ->kind('ternary_full')
            ->all();

        $this->assertNotEmpty($ternaryFullCalls, 'Full ternary operators should be present with --experimental flag');

        foreach ($ternaryFullCalls as $call) {
            $this->assertArrayHasKey(
                'condition_value_id',
                $call,
                sprintf('Ternary %s should have condition_value_id', $call['id'])
            );
            $this->assertArrayHasKey(
                'true_value_id',
                $call,
                sprintf('Ternary %s should have true_value_id', $call['id'])
            );
            $this->assertArrayHasKey(
                'false_value_id',
                $call,
                sprintf('Ternary %s should have false_value_id', $call['id'])
            );
        }
    }

    #[ContractTest(
        name: 'Short Ternary Has Condition ID',
        description: 'Verifies short ternary ($a ?: $b) has condition_value_id. True value is the condition itself.',
        category: 'operator',
        experimental: true,
    )]
    public function testShortTernaryHasConditionId(): void
    {
        $ternaryCalls = $this->calls()
            ->kind('ternary')
            ->all();

        $this->assertNotEmpty($ternaryCalls, 'Short ternary operators should be present with --experimental flag');

        foreach ($ternaryCalls as $call) {
            $this->assertArrayHasKey(
                'condition_value_id',
                $call,
                sprintf('Short ternary %s should have condition_value_id', $call['id'])
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Match Expression
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Match Expression Kind Exists',
        description: 'Verifies match expressions are tracked with kind=match, kind_type=operator, subject_value_id, and arm_ids.',
        category: 'operator',
        experimental: true,
    )]
    public function testMatchExpressionKindExists(): void
    {
        $matchCalls = $this->calls()
            ->kind('match')
            ->all();

        $this->assertNotEmpty(
            $matchCalls,
            'Match expressions should be present with --experimental flag'
        );

        // Verify structure
        $call = $matchCalls[0];
        $this->assertSame('operator', $call['kind_type'] ?? '');
        $this->assertArrayHasKey('subject_value_id', $call, 'Match should have subject_value_id');
        $this->assertArrayHasKey('arm_ids', $call, 'Match should have arm_ids array');
    }

    #[ContractTest(
        name: 'Match Expression Arms Reference Values',
        description: 'Verifies match expression arm_ids array contains valid value references for each arm result.',
        category: 'operator',
        experimental: true,
    )]
    public function testMatchExpressionArmsReferenceValues(): void
    {
        $matchCalls = $this->calls()
            ->kind('match')
            ->all();

        $this->assertNotEmpty($matchCalls, 'Match expressions should be present with --experimental flag');

        foreach ($matchCalls as $call) {
            $armIds = $call['arm_ids'] ?? [];
            $this->assertIsArray($armIds, 'arm_ids should be an array');

            foreach ($armIds as $armId) {
                $this->assertTrue(
                    self::$calls->hasValue($armId),
                    sprintf('Match %s arm_id %s should reference existing value', $call['id'], $armId)
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Operator Kind Type Validation
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Operators Have Kind Type Operator',
        description: 'Verifies all operator kinds (coalesce, ternary, ternary_full, match) have kind_type=operator.',
        category: 'operator',
        experimental: true,
    )]
    public function testAllOperatorsHaveKindTypeOperator(): void
    {
        $operatorKinds = ['coalesce', 'ternary', 'ternary_full', 'match'];

        $operatorCalls = [];
        foreach ($operatorKinds as $kind) {
            $calls = $this->calls()->kind($kind)->all();
            $operatorCalls = array_merge($operatorCalls, $calls);
        }

        if (empty($operatorCalls)) {
            $this->markTestSkipped('No operator calls found in the index');
        }

        $wrongKindType = [];
        foreach ($operatorCalls as $call) {
            if (($call['kind_type'] ?? '') !== 'operator') {
                $wrongKindType[] = sprintf(
                    '%s (kind=%s) has kind_type=%s',
                    $call['id'] ?? 'unknown',
                    $call['kind'] ?? 'unknown',
                    $call['kind_type'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $wrongKindType,
            sprintf(
                "Found %d operator calls with wrong kind_type:\n%s",
                count($wrongKindType),
                implode("\n", $wrongKindType)
            )
        );
    }

    #[ContractTest(
        name: 'Operators Have Result Values',
        description: 'Verifies operator calls have corresponding result values for data flow tracking.',
        category: 'operator',
        experimental: true,
    )]
    public function testOperatorsHaveResultValues(): void
    {
        $operatorKinds = ['coalesce', 'ternary', 'ternary_full', 'match'];

        $operatorCalls = [];
        foreach ($operatorKinds as $kind) {
            $calls = $this->calls()->kind($kind)->all();
            $operatorCalls = array_merge($operatorCalls, $calls);
        }

        if (empty($operatorCalls)) {
            $this->markTestSkipped('No operator calls found in the index');
        }

        $missingResults = [];
        foreach ($operatorCalls as $call) {
            $resultValue = self::$calls->getValueById($call['id']);
            if ($resultValue === null) {
                $missingResults[] = sprintf(
                    '%s (kind=%s)',
                    $call['id'] ?? 'unknown',
                    $call['kind'] ?? 'unknown'
                );
            }
        }

        $this->assertEmpty(
            $missingResults,
            sprintf(
                "Found %d operator calls without result values:\n%s",
                count($missingResults),
                implode("\n", $missingResults)
            )
        );
    }
}
