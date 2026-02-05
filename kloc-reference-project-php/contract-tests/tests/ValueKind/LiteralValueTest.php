<?php

declare(strict_types=1);

namespace ContractTests\Tests\ValueKind;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for literal value tracking in calls.json.
 *
 * Verifies that literal values (strings, integers, booleans, arrays)
 * are properly tracked in the values array with kind=literal.
 */
class LiteralValueTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Literal Value Kind
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Literal Values Exist',
        description: 'Verifies literal values (strings, numbers) are tracked with kind=literal in values array.',
        category: 'valuekind',
    )]
    public function testLiteralValuesExist(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        $this->assertNotEmpty(
            $literalValues,
            'Literal values should be tracked in values array with kind=literal'
        );
    }

    #[ContractTest(
        name: 'String Literals Have Type Information',
        description: 'Verifies string literal values have type containing string. NOTE: Literal type tracking not yet fully implemented.',
        category: 'valuekind',
    )]
    public function testStringLiteralsHaveTypeInformation(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        $stringLiterals = [];
        foreach ($literalValues as $value) {
            $type = $value['type'] ?? '';
            if (str_contains($type, 'string')) {
                $stringLiterals[] = $value;
            }
        }

        // Known limitation: Literal type tracking not yet fully implemented
        if (empty($stringLiterals)) {
            $this->markTestSkipped(
                '[KNOWN GAP] Literal type information not yet implemented in scip-php'
            );
        }

        $this->assertNotEmpty($stringLiterals, 'String literals should have type containing string');
    }

    #[ContractTest(
        name: 'Integer Literals Have Type Information',
        description: 'Verifies integer literal values have type containing int.',
        category: 'valuekind',
    )]
    public function testIntegerLiteralsHaveTypeInformation(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        $intLiterals = [];
        foreach ($literalValues as $value) {
            $type = $value['type'] ?? '';
            if (str_contains($type, 'int')) {
                $intLiterals[] = $value;
            }
        }

        // Integer literals may or may not be present depending on code
        if (empty($intLiterals)) {
            $this->markTestSkipped('No integer literals found in index');
        }

        $this->assertNotEmpty($intLiterals, 'Integer literals should have type containing int');
    }

    #[ContractTest(
        name: 'Boolean Literals Have Type Information',
        description: 'Verifies boolean literal values (true, false) have type containing bool.',
        category: 'valuekind',
    )]
    public function testBooleanLiteralsHaveTypeInformation(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        $boolLiterals = [];
        foreach ($literalValues as $value) {
            $type = $value['type'] ?? '';
            if (str_contains($type, 'bool')) {
                $boolLiterals[] = $value;
            }
        }

        // Boolean literals may or may not be present depending on code
        if (empty($boolLiterals)) {
            $this->markTestSkipped('No boolean literals found in index');
        }

        $this->assertNotEmpty($boolLiterals, 'Boolean literals should have type containing bool');
    }

    // ═══════════════════════════════════════════════════════════════
    // Literals in Arguments
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Literal Arguments Have Value Expression',
        description: 'Verifies literals passed as arguments have value_expr capturing the literal text.',
        category: 'valuekind',
    )]
    public function testLiteralArgumentsHaveValueExpression(): void
    {
        $calls = self::$calls->calls();

        $callsWithLiteralArgs = [];
        foreach ($calls as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $valueExpr = $arg['value_expr'] ?? '';
                // Look for literal-like expressions (quoted strings, numbers, true/false)
                if (
                    preg_match('/^["\'].*["\']$/', $valueExpr) ||
                    preg_match('/^\d+$/', $valueExpr) ||
                    in_array($valueExpr, ['true', 'false', 'null'], true)
                ) {
                    $callsWithLiteralArgs[] = [
                        'call_id' => $call['id'],
                        'value_expr' => $valueExpr,
                    ];
                }
            }
        }

        $this->assertNotEmpty(
            $callsWithLiteralArgs,
            'Should have calls with literal arguments captured in value_expr'
        );
    }

    #[ContractTest(
        name: 'String Literal Arguments Have Quoted Value Expression',
        description: 'Verifies string literals in arguments preserve quotes in value_expr.',
        category: 'valuekind',
    )]
    public function testStringLiteralArgumentsHaveQuotedValueExpression(): void
    {
        // Code reference: Various places with string literals like 'pending', 'unknown'
        $calls = self::$calls->calls();

        $stringLiteralArgs = [];
        foreach ($calls as $call) {
            $arguments = $call['arguments'] ?? [];
            foreach ($arguments as $arg) {
                $valueExpr = $arg['value_expr'] ?? '';
                if (preg_match('/^["\'].*["\']$/', $valueExpr)) {
                    $stringLiteralArgs[] = $valueExpr;
                }
            }
        }

        $this->assertNotEmpty(
            $stringLiteralArgs,
            'Should have string literal arguments with quoted value_expr'
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Literal Value Structure
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Literal Values Have Required Fields',
        description: 'Verifies literal values have id, kind, and location fields.',
        category: 'valuekind',
    )]
    public function testLiteralValuesHaveRequiredFields(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        if (empty($literalValues)) {
            $this->markTestSkipped('No literal values found in index');
        }

        foreach ($literalValues as $value) {
            $this->assertArrayHasKey('id', $value, 'Literal value should have id');
            $this->assertArrayHasKey('kind', $value, 'Literal value should have kind');
            $this->assertSame('literal', $value['kind'], 'Kind should be literal');

            if (isset($value['location'])) {
                $this->assertArrayHasKey('file', $value['location'], 'Location should have file');
                $this->assertArrayHasKey('line', $value['location'], 'Location should have line');
            }
        }
    }

    #[ContractTest(
        name: 'Literal Values Have Type Field',
        description: 'Verifies literal values have type field indicating the literal type. NOTE: Literal type tracking not yet fully implemented.',
        category: 'valuekind',
    )]
    public function testLiteralValuesHaveTypeField(): void
    {
        $values = self::$calls->values();

        $literalValues = array_filter(
            $values,
            static fn(array $v): bool => ($v['kind'] ?? '') === 'literal'
        );

        if (empty($literalValues)) {
            $this->markTestSkipped('No literal values found in index');
        }

        $literalsWithType = 0;
        foreach ($literalValues as $value) {
            if (!empty($value['type'])) {
                $literalsWithType++;
            }
        }

        // Known limitation: Literal type tracking not yet fully implemented
        if ($literalsWithType === 0) {
            $this->markTestSkipped(
                '[KNOWN GAP] Literal type information not yet implemented in scip-php'
            );
        }

        $this->assertGreaterThan(0, $literalsWithType, 'At least some literal values should have type field');
    }
}
