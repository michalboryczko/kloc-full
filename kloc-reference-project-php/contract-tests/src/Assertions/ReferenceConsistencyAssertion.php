<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

use ContractTests\CallsData;
use ContractTests\Query\ValueQuery;
use ContractTests\Query\CallQuery;
use PHPUnit\Framework\Assert;
use PHPUnit\Framework\TestCase;

/**
 * Assertion builder for verifying reference consistency.
 *
 * Reference consistency means:
 * - A parameter/local has exactly one value entry at its declaration
 * - All usages of that variable reference the same value ID
 *
 * Usage:
 *   $this->assertReferenceConsistency()
 *       ->inMethod('App\Repository\OrderRepository', 'save')
 *       ->forParameter('$order')
 *       ->verify();
 */
final class ReferenceConsistencyAssertion
{
    private CallsData $data;
    private TestCase $testCase;
    private ?string $class = null;
    private ?string $method = null;
    private ?string $variableName = null;
    private string $variableKind = 'parameter';

    public function __construct(CallsData $data, TestCase $testCase)
    {
        $this->data = $data;
        $this->testCase = $testCase;
    }

    /**
     * Scope to a specific method.
     */
    public function inMethod(string $class, string $method): self
    {
        $clone = clone $this;
        $clone->class = ltrim($class, '\\');
        $clone->method = $method;
        return $clone;
    }

    /**
     * Check consistency for a parameter.
     *
     * @param string $name Parameter name including $ (e.g., '$order')
     */
    public function forParameter(string $name): self
    {
        $clone = clone $this;
        $clone->variableName = $name;
        $clone->variableKind = 'parameter';
        return $clone;
    }

    /**
     * Check consistency for a local variable.
     *
     * @param string $name Variable name including $ (e.g., '$newOrder')
     */
    public function forLocal(string $name): self
    {
        $clone = clone $this;
        $clone->variableName = $name;
        $clone->variableKind = 'local';
        return $clone;
    }

    /**
     * Run the verification.
     *
     * Checks:
     * - Exactly one value entry exists for the variable
     * - All calls using this variable as receiver reference that value ID
     *
     * @return VerificationResult
     */
    public function verify(): VerificationResult
    {
        $this->validateConfiguration();

        // Build scope pattern for symbol matching
        $scipClass = str_replace('\\', '/', $this->class);
        $scopePattern = $scipClass . '#' . $this->method . '()';

        // Find the value entry
        $query = (new ValueQuery($this->data))
            ->kind($this->variableKind);

        if ($this->variableKind === 'parameter') {
            // Parameter symbol format: ...#method().($paramName)
            $symbolPattern = $scopePattern . '.(' . $this->variableName . ')';
            $query = $query->symbolContains($symbolPattern);
        } else {
            // Local symbol format: ...#method().local$varName@line
            // We match without the @line since we don't know the line
            $varNameWithoutDollar = substr($this->variableName, 1);
            $symbolPattern = $scopePattern . '.local$' . $varNameWithoutDollar;
            $query = $query->symbolContains($symbolPattern);
        }

        $values = $query->all();

        // For locals, there may be multiple if re-assigned on different lines
        // For parameters, there should be exactly one
        if ($this->variableKind === 'parameter') {
            Assert::assertCount(
                1,
                $values,
                sprintf(
                    'Expected exactly 1 value entry for parameter %s in %s::%s, found %d. ' .
                    'Symbol pattern: %s',
                    $this->variableName,
                    $this->class,
                    $this->method,
                    count($values),
                    $symbolPattern
                )
            );
            $valueId = $values[0]['id'];
        } else {
            // For locals, at least one should exist
            Assert::assertNotEmpty(
                $values,
                sprintf(
                    'No value entry found for local %s in %s::%s. Symbol pattern: %s',
                    $this->variableName,
                    $this->class,
                    $this->method,
                    $symbolPattern
                )
            );
            // Use the first one (earliest declaration)
            $valueId = $values[0]['id'];
        }

        // Find all calls that use this variable as receiver
        $callsQuery = (new CallQuery($this->data))
            ->callerContains($scopePattern)
            ->hasReceiver();

        $calls = $callsQuery->all();

        // Check that all calls with receiver_value_id pointing to our value exist
        $callsWithThisReceiver = array_filter(
            $calls,
            fn($c) => ($c['receiver_value_id'] ?? '') === $valueId
        );

        return new VerificationResult(
            success: true,
            valueId: $valueId,
            valueCount: count($values),
            callCount: count($callsWithThisReceiver),
            message: sprintf(
                'Reference consistency verified for %s %s in %s::%s: ' .
                '1 value entry (id=%s), %d calls reference it',
                $this->variableKind,
                $this->variableName,
                $this->class,
                $this->method,
                $valueId,
                count($callsWithThisReceiver)
            )
        );
    }

    private function validateConfiguration(): void
    {
        if ($this->class === null || $this->method === null) {
            throw new \InvalidArgumentException(
                'Must call inMethod() before verify()'
            );
        }

        if ($this->variableName === null) {
            throw new \InvalidArgumentException(
                'Must call forParameter() or forLocal() before verify()'
            );
        }
    }
}

/**
 * Result of reference consistency verification.
 */
final class VerificationResult
{
    public function __construct(
        public readonly bool $success,
        public readonly string $valueId,
        public readonly int $valueCount,
        public readonly int $callCount,
        public readonly string $message,
    ) {
    }
}
