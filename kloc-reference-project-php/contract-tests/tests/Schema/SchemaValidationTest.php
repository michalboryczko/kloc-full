<?php

declare(strict_types=1);

namespace ContractTests\Tests\Schema;

use ContractTests\Attribute\ContractTest;
use ContractTests\CallsContractTestCase;

/**
 * Tests for JSON schema validation of calls.json structure.
 *
 * Validates that the generated calls.json conforms to the expected schema
 * defined in docs/reference/kloc-scip/calls-schema.json.
 */
class SchemaValidationTest extends CallsContractTestCase
{
    // ═══════════════════════════════════════════════════════════════
    // Top-Level Structure
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Version Format Valid',
        description: 'Verifies version field matches semver pattern (e.g., "3.2"). Schema requires pattern ^[0-9]+\.[0-9]+(\.[0-9]+)?$',
        category: 'schema',
    )]
    public function testVersionFormatValid(): void
    {
        $version = self::$calls->version();

        $this->assertNotEmpty($version, 'Version should be present');
        $this->assertMatchesRegularExpression(
            '/^\d+\.\d+(\.\d+)?$/',
            $version,
            sprintf('Version "%s" should match semver pattern (e.g., 3.2 or 3.2.1)', $version)
        );
    }

    #[ContractTest(
        name: 'Values Array Present and Non-Empty',
        description: 'Verifies values array exists and contains entries. Required per schema.',
        category: 'schema',
    )]
    public function testValuesArrayPresentAndNonEmpty(): void
    {
        $values = self::$calls->values();

        $this->assertIsArray($values, 'Values should be an array');
        $this->assertNotEmpty($values, 'Values array should not be empty');
    }

    #[ContractTest(
        name: 'Calls Array Present and Non-Empty',
        description: 'Verifies calls array exists and contains entries. Required per schema.',
        category: 'schema',
    )]
    public function testCallsArrayPresentAndNonEmpty(): void
    {
        $calls = self::$calls->calls();

        $this->assertIsArray($calls, 'Calls should be an array');
        $this->assertNotEmpty($calls, 'Calls array should not be empty');
    }

    // ═══════════════════════════════════════════════════════════════
    // Value Record Structure
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Values Have Required Fields',
        description: 'Verifies every value record has required fields: id, kind, location. Per schema ValueRecord definition.',
        category: 'schema',
    )]
    public function testAllValuesHaveRequiredFields(): void
    {
        $invalidValues = [];

        foreach (self::$calls->values() as $index => $value) {
            $missing = [];

            if (!isset($value['id'])) {
                $missing[] = 'id';
            }
            if (!isset($value['kind'])) {
                $missing[] = 'kind';
            }
            if (!isset($value['location'])) {
                $missing[] = 'location';
            }

            if (!empty($missing)) {
                $invalidValues[] = sprintf(
                    'Value at index %d (id: %s) missing: %s',
                    $index,
                    $value['id'] ?? 'unknown',
                    implode(', ', $missing)
                );
            }
        }

        $this->assertEmpty(
            $invalidValues,
            "Found values with missing required fields:\n" . implode("\n", array_slice($invalidValues, 0, 10))
        );
    }

    #[ContractTest(
        name: 'All Value IDs Follow Location Format',
        description: 'Verifies value IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition.',
        category: 'schema',
    )]
    public function testAllValueIdsFollowLocationFormat(): void
    {
        $invalidIds = [];

        foreach (self::$calls->values() as $value) {
            $id = $value['id'] ?? '';
            // Pattern: file:line:col where line and col are integers
            if (!preg_match('/^[^:]+:\d+:\d+$/', $id)) {
                $invalidIds[] = $id;
            }
        }

        $this->assertEmpty(
            $invalidIds,
            sprintf(
                'Found %d value IDs not matching {file}:{line}:{col} pattern: %s',
                count($invalidIds),
                implode(', ', array_slice($invalidIds, 0, 5))
            )
        );
    }

    #[ContractTest(
        name: 'All Value Kinds Are Valid',
        description: 'Verifies every value kind is one of: parameter, local, literal, constant, result. Per schema ValueKind enum.',
        category: 'schema',
    )]
    public function testAllValueKindsAreValid(): void
    {
        $validKinds = ['parameter', 'local', 'literal', 'constant', 'result'];
        $invalidValues = [];

        foreach (self::$calls->values() as $value) {
            $kind = $value['kind'] ?? '';
            if (!in_array($kind, $validKinds, true)) {
                $invalidValues[] = sprintf(
                    'Value %s has invalid kind: "%s"',
                    $value['id'] ?? 'unknown',
                    $kind
                );
            }
        }

        $this->assertEmpty(
            $invalidValues,
            sprintf(
                "Found values with invalid kinds (valid: %s):\n%s",
                implode(', ', $validKinds),
                implode("\n", array_slice($invalidValues, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Value Locations Have Required Fields',
        description: 'Verifies every value location has file, line, and col fields. Per schema Location definition.',
        category: 'schema',
    )]
    public function testValueLocationsHaveRequiredFields(): void
    {
        $invalidLocations = [];

        foreach (self::$calls->values() as $value) {
            $location = $value['location'] ?? null;

            if ($location === null) {
                continue; // Already caught by required fields test
            }

            $missing = [];
            if (!isset($location['file'])) {
                $missing[] = 'file';
            }
            if (!isset($location['line'])) {
                $missing[] = 'line';
            }
            if (!isset($location['col'])) {
                $missing[] = 'col';
            }

            if (!empty($missing)) {
                $invalidLocations[] = sprintf(
                    'Value %s location missing: %s',
                    $value['id'] ?? 'unknown',
                    implode(', ', $missing)
                );
            }
        }

        $this->assertEmpty(
            $invalidLocations,
            "Found values with incomplete location:\n" . implode("\n", array_slice($invalidLocations, 0, 10))
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Call Record Structure
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Calls Have Required Fields',
        description: 'Verifies every call record has required fields: id, kind, kind_type, caller, location. Per schema CallRecord definition.',
        category: 'schema',
    )]
    public function testAllCallsHaveRequiredFields(): void
    {
        $invalidCalls = [];

        foreach (self::$calls->calls() as $index => $call) {
            $missing = [];

            if (!isset($call['id'])) {
                $missing[] = 'id';
            }
            if (!isset($call['kind'])) {
                $missing[] = 'kind';
            }
            if (!isset($call['kind_type'])) {
                $missing[] = 'kind_type';
            }
            if (!isset($call['caller'])) {
                $missing[] = 'caller';
            }
            if (!isset($call['location'])) {
                $missing[] = 'location';
            }

            if (!empty($missing)) {
                $invalidCalls[] = sprintf(
                    'Call at index %d (id: %s) missing: %s',
                    $index,
                    $call['id'] ?? 'unknown',
                    implode(', ', $missing)
                );
            }
        }

        $this->assertEmpty(
            $invalidCalls,
            "Found calls with missing required fields:\n" . implode("\n", array_slice($invalidCalls, 0, 10))
        );
    }

    #[ContractTest(
        name: 'All Call IDs Follow Location Format',
        description: 'Verifies call IDs match LocationId pattern: {file}:{line}:{col}. Per schema LocationId definition.',
        category: 'schema',
    )]
    public function testAllCallIdsFollowLocationFormat(): void
    {
        $invalidIds = [];

        foreach (self::$calls->calls() as $call) {
            $id = $call['id'] ?? '';
            if (!preg_match('/^[^:]+:\d+:\d+$/', $id)) {
                $invalidIds[] = $id;
            }
        }

        $this->assertEmpty(
            $invalidIds,
            sprintf(
                'Found %d call IDs not matching {file}:{line}:{col} pattern: %s',
                count($invalidIds),
                implode(', ', array_slice($invalidIds, 0, 5))
            )
        );
    }

    #[ContractTest(
        name: 'All Call Kinds Are Valid',
        description: 'Verifies every call kind is one of the defined enum values. Per schema CallKind enum.',
        category: 'schema',
    )]
    public function testAllCallKindsAreValid(): void
    {
        $validKinds = [
            'method', 'method_static', 'method_nullsafe',
            'function', 'constructor',
            'access', 'access_static', 'access_nullsafe', 'access_array',
            'coalesce', 'ternary', 'ternary_full', 'match',
        ];
        $invalidCalls = [];

        foreach (self::$calls->calls() as $call) {
            $kind = $call['kind'] ?? '';
            if (!in_array($kind, $validKinds, true)) {
                $invalidCalls[] = sprintf(
                    'Call %s has invalid kind: "%s"',
                    $call['id'] ?? 'unknown',
                    $kind
                );
            }
        }

        $this->assertEmpty(
            $invalidCalls,
            sprintf(
                "Found calls with invalid kinds (valid: %s):\n%s",
                implode(', ', $validKinds),
                implode("\n", array_slice($invalidCalls, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'All Call Kind Types Are Valid',
        description: 'Verifies every call kind_type is one of: invocation, access, operator. Per schema CallKindType enum.',
        category: 'schema',
    )]
    public function testAllCallKindTypesAreValid(): void
    {
        $validKindTypes = ['invocation', 'access', 'operator'];
        $invalidCalls = [];

        foreach (self::$calls->calls() as $call) {
            $kindType = $call['kind_type'] ?? '';
            if (!in_array($kindType, $validKindTypes, true)) {
                $invalidCalls[] = sprintf(
                    'Call %s has invalid kind_type: "%s"',
                    $call['id'] ?? 'unknown',
                    $kindType
                );
            }
        }

        $this->assertEmpty(
            $invalidCalls,
            sprintf(
                "Found calls with invalid kind_types (valid: %s):\n%s",
                implode(', ', $validKindTypes),
                implode("\n", array_slice($invalidCalls, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Call Kind Type Matches Kind Category',
        description: 'Verifies kind_type correctly categorizes each kind. Methods/functions/constructors = invocation, property/array access = access, operators = operator.',
        category: 'schema',
    )]
    public function testCallKindTypeMatchesKindCategory(): void
    {
        $kindToKindType = [
            'method' => 'invocation',
            'method_static' => 'invocation',
            'method_nullsafe' => 'invocation',
            'function' => 'invocation',
            'constructor' => 'invocation',
            'access' => 'access',
            'access_static' => 'access',
            'access_nullsafe' => 'access',
            'access_array' => 'access',
            'coalesce' => 'operator',
            'ternary' => 'operator',
            'ternary_full' => 'operator',
            'match' => 'operator',
        ];

        $mismatched = [];

        foreach (self::$calls->calls() as $call) {
            $kind = $call['kind'] ?? '';
            $kindType = $call['kind_type'] ?? '';

            if (isset($kindToKindType[$kind]) && $kindToKindType[$kind] !== $kindType) {
                $mismatched[] = sprintf(
                    'Call %s: kind=%s should have kind_type=%s, got %s',
                    $call['id'] ?? 'unknown',
                    $kind,
                    $kindToKindType[$kind],
                    $kindType
                );
            }
        }

        $this->assertEmpty(
            $mismatched,
            "Found calls with mismatched kind_type:\n" . implode("\n", array_slice($mismatched, 0, 10))
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Argument Record Structure
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'All Arguments Have Position',
        description: 'Verifies every argument record has a position field. Per schema ArgumentRecord definition.',
        category: 'schema',
    )]
    public function testAllArgumentsHavePosition(): void
    {
        $invalidArgs = [];

        foreach (self::$calls->calls() as $call) {
            $arguments = $call['arguments'] ?? [];

            foreach ($arguments as $argIndex => $arg) {
                if (!isset($arg['position'])) {
                    $invalidArgs[] = sprintf(
                        'Call %s argument at index %d missing position',
                        $call['id'] ?? 'unknown',
                        $argIndex
                    );
                }
            }
        }

        $this->assertEmpty(
            $invalidArgs,
            "Found arguments missing position field:\n" . implode("\n", array_slice($invalidArgs, 0, 10))
        );
    }

    #[ContractTest(
        name: 'Argument Positions Are Zero-Based',
        description: 'Verifies argument positions are 0-indexed and sequential for each call.',
        category: 'schema',
    )]
    public function testArgumentPositionsAreZeroBased(): void
    {
        $issues = [];

        foreach (self::$calls->calls() as $call) {
            $arguments = $call['arguments'] ?? [];

            if (empty($arguments)) {
                continue;
            }

            $positions = array_column($arguments, 'position');
            sort($positions);

            // Check first position is 0
            if (!empty($positions) && $positions[0] !== 0) {
                $issues[] = sprintf(
                    'Call %s first argument position is %d, expected 0',
                    $call['id'] ?? 'unknown',
                    $positions[0]
                );
            }
        }

        $this->assertEmpty(
            $issues,
            "Found calls with non-zero-based argument positions:\n" . implode("\n", array_slice($issues, 0, 10))
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // ID Uniqueness
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Value IDs Are Unique',
        description: 'Verifies all value IDs are unique within the values array (no duplicates).',
        category: 'schema',
    )]
    public function testValueIdsAreUnique(): void
    {
        $ids = [];
        $duplicates = [];

        foreach (self::$calls->values() as $value) {
            $id = $value['id'] ?? '';
            if (isset($ids[$id])) {
                $duplicates[] = $id;
            }
            $ids[$id] = true;
        }

        $this->assertEmpty(
            $duplicates,
            sprintf(
                "Found %d duplicate value IDs:\n%s",
                count($duplicates),
                implode("\n", array_slice($duplicates, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Call IDs Are Unique',
        description: 'Verifies all call IDs are unique within the calls array (no duplicates).',
        category: 'schema',
    )]
    public function testCallIdsAreUnique(): void
    {
        $ids = [];
        $duplicates = [];

        foreach (self::$calls->calls() as $call) {
            $id = $call['id'] ?? '';
            if (isset($ids[$id])) {
                $duplicates[] = $id;
            }
            $ids[$id] = true;
        }

        $this->assertEmpty(
            $duplicates,
            sprintf(
                "Found %d duplicate call IDs:\n%s",
                count($duplicates),
                implode("\n", array_slice($duplicates, 0, 10))
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Call Locations Match IDs
    // ═══════════════════════════════════════════════════════════════

    #[ContractTest(
        name: 'Call ID Matches Location',
        description: 'Verifies call ID is consistent with location (file:line:col format).',
        category: 'schema',
    )]
    public function testCallIdMatchesLocation(): void
    {
        $mismatched = [];

        foreach (self::$calls->calls() as $call) {
            $id = $call['id'] ?? '';
            $location = $call['location'] ?? [];

            if (empty($location)) {
                continue;
            }

            $expectedId = sprintf(
                '%s:%d:%d',
                $location['file'] ?? '',
                $location['line'] ?? 0,
                $location['col'] ?? 0
            );

            if ($id !== $expectedId) {
                $mismatched[] = sprintf(
                    'ID %s does not match location %s',
                    $id,
                    $expectedId
                );
            }
        }

        $this->assertEmpty(
            $mismatched,
            sprintf(
                "Found %d calls where ID doesn't match location:\n%s",
                count($mismatched),
                implode("\n", array_slice($mismatched, 0, 10))
            )
        );
    }

    #[ContractTest(
        name: 'Value ID Matches Location',
        description: 'Verifies value ID is consistent with location (file:line:col format).',
        category: 'schema',
    )]
    public function testValueIdMatchesLocation(): void
    {
        $mismatched = [];

        foreach (self::$calls->values() as $value) {
            $id = $value['id'] ?? '';
            $location = $value['location'] ?? [];

            if (empty($location)) {
                continue;
            }

            $expectedId = sprintf(
                '%s:%d:%d',
                $location['file'] ?? '',
                $location['line'] ?? 0,
                $location['col'] ?? 0
            );

            if ($id !== $expectedId) {
                $mismatched[] = sprintf(
                    'ID %s does not match location %s',
                    $id,
                    $expectedId
                );
            }
        }

        $this->assertEmpty(
            $mismatched,
            sprintf(
                "Found %d values where ID doesn't match location:\n%s",
                count($mismatched),
                implode("\n", array_slice($mismatched, 0, 10))
            )
        );
    }
}
