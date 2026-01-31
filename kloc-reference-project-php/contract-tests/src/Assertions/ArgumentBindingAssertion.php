<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

use ContractTests\CallsData;
use ContractTests\Query\CallQuery;
use ContractTests\Query\ValueQuery;
use PHPUnit\Framework\Assert;
use PHPUnit\Framework\TestCase;

/**
 * Assertion builder for verifying argument bindings.
 *
 * Argument binding means:
 * - An argument's value_id points to the correct value
 * - The value has the expected kind (parameter, local, result, etc.)
 *
 * Usage:
 *   $this->assertArgument()
 *       ->inMethod('App\Service\OrderService', 'createOrder')
 *       ->atCall('save')
 *       ->position(0)
 *       ->pointsToLocal('$order')
 *       ->verify();
 */
final class ArgumentBindingAssertion
{
    private CallsData $data;
    private TestCase $testCase;
    private ?string $class = null;
    private ?string $method = null;
    private ?string $callee = null;
    private ?int $line = null;
    private ?int $position = null;

    private ?string $expectedKind = null;
    private ?string $expectedName = null;
    private ?string $expectedResultKind = null;
    private ?string $expectedResultCallee = null;

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
     * Select call by callee name.
     *
     * @param string $callee Method/function being called (e.g., 'validate', 'save')
     */
    public function atCall(string $callee): self
    {
        $clone = clone $this;
        $clone->callee = $callee;
        return $clone;
    }

    /**
     * Select call by line number.
     */
    public function atLine(int $line): self
    {
        $clone = clone $this;
        $clone->line = $line;
        return $clone;
    }

    /**
     * Select argument by position (0-based).
     */
    public function position(int $pos): self
    {
        $clone = clone $this;
        $clone->position = $pos;
        return $clone;
    }

    /**
     * Assert argument points to a parameter value.
     *
     * @param string $paramName Parameter name (e.g., '$order')
     */
    public function pointsToParameter(string $paramName): self
    {
        $clone = clone $this;
        $clone->expectedKind = 'parameter';
        $clone->expectedName = $paramName;
        return $clone;
    }

    /**
     * Assert argument points to a local value.
     *
     * @param string $localName Local variable name (e.g., '$validated')
     */
    public function pointsToLocal(string $localName): self
    {
        $clone = clone $this;
        $clone->expectedKind = 'local';
        $clone->expectedName = $localName;
        return $clone;
    }

    /**
     * Assert argument points to a literal value.
     */
    public function pointsToLiteral(): self
    {
        $clone = clone $this;
        $clone->expectedKind = 'literal';
        $clone->expectedName = null;
        return $clone;
    }

    /**
     * Assert argument points to a result value from specified call.
     *
     * @param string $kind Call kind (e.g., 'access', 'method')
     * @param string $calleePattern Callee pattern (e.g., '*#email.', 'customerEmail')
     */
    public function pointsToResultOf(string $kind, string $calleePattern): self
    {
        $clone = clone $this;
        $clone->expectedKind = 'result';
        $clone->expectedResultKind = $kind;
        $clone->expectedResultCallee = $calleePattern;
        return $clone;
    }

    /**
     * Run the verification.
     */
    public function verify(): void
    {
        $this->validateConfiguration();

        $call = $this->findCall();
        Assert::assertNotNull(
            $call,
            $this->buildCallNotFoundMessage()
        );

        $arguments = $call['arguments'] ?? [];
        Assert::assertNotEmpty(
            $arguments,
            sprintf(
                'Call to %s has no arguments, but expected argument at position %d',
                $this->callee,
                $this->position
            )
        );

        // Find argument at position
        $argument = null;
        foreach ($arguments as $arg) {
            if (($arg['position'] ?? -1) === $this->position) {
                $argument = $arg;
                break;
            }
        }

        Assert::assertNotNull(
            $argument,
            sprintf(
                'No argument found at position %d for call to %s. Available positions: %s',
                $this->position,
                $this->callee,
                implode(', ', array_column($arguments, 'position'))
            )
        );

        $valueId = $argument['value_id'] ?? null;
        Assert::assertNotNull(
            $valueId,
            sprintf(
                'Argument at position %d has no value_id',
                $this->position
            )
        );

        // Look up the value
        $value = $this->data->getValueById($valueId);
        Assert::assertNotNull(
            $value,
            sprintf(
                'Argument value_id %s does not exist in values array',
                $valueId
            )
        );

        // Verify value kind
        $actualKind = $value['kind'] ?? 'unknown';
        Assert::assertEquals(
            $this->expectedKind,
            $actualKind,
            sprintf(
                'Argument at position %d points to %s value (id=%s), expected %s',
                $this->position,
                $actualKind,
                $valueId,
                $this->expectedKind
            )
        );

        // For parameter/local, verify the name matches
        if ($this->expectedName !== null && in_array($this->expectedKind, ['parameter', 'local'])) {
            $symbol = $value['symbol'] ?? '';
            $varNameWithoutDollar = substr($this->expectedName, 1);

            if ($this->expectedKind === 'parameter') {
                Assert::assertStringContainsString(
                    '($' . $varNameWithoutDollar . ')',
                    $symbol,
                    sprintf(
                        'Argument at position %d points to value with symbol "%s", expected to contain parameter %s',
                        $this->position,
                        $symbol,
                        $this->expectedName
                    )
                );
            } else {
                Assert::assertStringContainsString(
                    'local$' . $varNameWithoutDollar,
                    $symbol,
                    sprintf(
                        'Argument at position %d points to value with symbol "%s", expected to contain local %s',
                        $this->position,
                        $symbol,
                        $this->expectedName
                    )
                );
            }
        }

        // For result, verify source call matches
        if ($this->expectedKind === 'result' && $this->expectedResultCallee !== null) {
            $sourceCallId = $value['source_call_id'] ?? null;
            Assert::assertNotNull(
                $sourceCallId,
                sprintf(
                    'Result value has no source_call_id (id=%s)',
                    $valueId
                )
            );

            $sourceCall = $this->data->getCallById($sourceCallId);
            Assert::assertNotNull(
                $sourceCall,
                sprintf(
                    'Result value source_call_id %s does not exist',
                    $sourceCallId
                )
            );

            if ($this->expectedResultKind !== null) {
                Assert::assertEquals(
                    $this->expectedResultKind,
                    $sourceCall['kind'] ?? 'unknown',
                    sprintf(
                        'Source call kind mismatch: expected %s, got %s',
                        $this->expectedResultKind,
                        $sourceCall['kind'] ?? 'unknown'
                    )
                );
            }

            Assert::assertStringContainsString(
                $this->expectedResultCallee,
                $sourceCall['callee'] ?? '',
                sprintf(
                    'Source call callee "%s" does not contain expected pattern "%s"',
                    $sourceCall['callee'] ?? '',
                    $this->expectedResultCallee
                )
            );
        }
    }

    /**
     * @return array<string, mixed>|null
     */
    private function findCall(): ?array
    {
        $scipClass = str_replace('\\', '/', $this->class);
        $scopePattern = $scipClass . '#' . $this->method . '()';

        $query = (new CallQuery($this->data))
            ->callerContains($scopePattern);

        if ($this->callee !== null) {
            $query = $query->calleeContains($this->callee);
        }

        if ($this->line !== null) {
            $query = $query->atLine($this->line);
        }

        $calls = $query->all();

        if (empty($calls)) {
            return null;
        }

        // If multiple calls match, take the first one
        return $calls[0];
    }

    private function buildCallNotFoundMessage(): string
    {
        $parts = [
            sprintf('Could not find call in %s::%s', $this->class, $this->method)
        ];

        if ($this->callee !== null) {
            $parts[] = sprintf('callee containing "%s"', $this->callee);
        }

        if ($this->line !== null) {
            $parts[] = sprintf('at line %d', $this->line);
        }

        return implode(' with ', $parts);
    }

    private function validateConfiguration(): void
    {
        if ($this->class === null || $this->method === null) {
            throw new \InvalidArgumentException(
                'Must call inMethod() before verify()'
            );
        }

        if ($this->callee === null && $this->line === null) {
            throw new \InvalidArgumentException(
                'Must call atCall() or atLine() before verify()'
            );
        }

        if ($this->position === null) {
            throw new \InvalidArgumentException(
                'Must call position() before verify()'
            );
        }

        if ($this->expectedKind === null) {
            throw new \InvalidArgumentException(
                'Must call pointsToParameter(), pointsToLocal(), pointsToLiteral(), or pointsToResultOf() before verify()'
            );
        }
    }
}
