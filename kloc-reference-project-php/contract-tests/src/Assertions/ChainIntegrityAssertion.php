<?php

declare(strict_types=1);

namespace ContractTests\Assertions;

use ContractTests\CallsData;
use ContractTests\Query\ValueQuery;
use ContractTests\Query\CallQuery;
use PHPUnit\Framework\Assert;
use PHPUnit\Framework\TestCase;

/**
 * Assertion builder for verifying chain integrity.
 *
 * Chain integrity means:
 * - Each step in a chain (value->call->value->call) is properly linked
 * - Each call's receiver_value_id points to the previous step's result
 * - Each result value's source_call_id points back to its call
 *
 * Usage:
 *   $this->assertChain()
 *       ->startingFrom('App\Service\OrderService', 'createOrder', '$this')
 *       ->throughAccess('orderRepository')
 *       ->throughMethod('save')
 *       ->verify();
 */
final class ChainIntegrityAssertion
{
    private CallsData $data;
    private TestCase $testCase;
    private ?string $class = null;
    private ?string $method = null;
    private ?string $startingVar = null;

    /** @var list<array{type: string, name: string}> */
    private array $steps = [];

    public function __construct(CallsData $data, TestCase $testCase)
    {
        $this->data = $data;
        $this->testCase = $testCase;
    }

    /**
     * Define chain starting point.
     *
     * @param string $class  Class containing the chain
     * @param string $method Method containing the chain
     * @param string $var    Starting variable (e.g., '$msg', '$this')
     */
    public function startingFrom(string $class, string $method, string $var): self
    {
        $clone = clone $this;
        $clone->class = ltrim($class, '\\');
        $clone->method = $method;
        $clone->startingVar = $var;
        return $clone;
    }

    /**
     * Add property access step to expected chain.
     *
     * @param string $property Property name (e.g., 'contact', 'orderRepository')
     */
    public function throughAccess(string $property): self
    {
        $clone = clone $this;
        $clone->steps[] = ['type' => 'access', 'name' => $property];
        return $clone;
    }

    /**
     * Add method call step to expected chain.
     *
     * @param string $method Method name (e.g., 'getProfile', 'save')
     */
    public function throughMethod(string $method): self
    {
        $clone = clone $this;
        $clone->steps[] = ['type' => 'method', 'name' => $method];
        return $clone;
    }

    /**
     * Run the verification.
     *
     * @return ChainVerificationResult
     */
    public function verify(): ChainVerificationResult
    {
        $this->validateConfiguration();

        $scipClass = str_replace('\\', '/', $this->class);
        $scopePattern = $scipClass . '#' . $this->method . '()';

        // Find the starting value
        $startValue = $this->findStartingValue($scopePattern);

        Assert::assertNotNull(
            $startValue,
            sprintf(
                'Could not find starting value for %s in %s::%s',
                $this->startingVar,
                $this->class,
                $this->method
            )
        );

        // Trace through the chain
        $currentValueId = $startValue['id'];
        $tracedSteps = [];
        $tracedSteps[] = ['type' => 'value', 'data' => $startValue];

        foreach ($this->steps as $expectedStep) {
            // Find the call that uses currentValueId as receiver
            $callKind = $expectedStep['type'];
            $callName = $expectedStep['name'];

            $callQuery = (new CallQuery($this->data))
                ->callerContains($scopePattern)
                ->withReceiverValueId($currentValueId);

            if ($callKind === 'access') {
                $callQuery = $callQuery->kind('access');
            } else {
                $callQuery = $callQuery->kind('method');
            }

            // Filter by callee containing the property/method name
            $calls = array_filter(
                $callQuery->all(),
                fn($c) => str_contains($c['callee'] ?? '', $callName)
            );

            Assert::assertNotEmpty(
                $calls,
                sprintf(
                    'Could not find %s call to "%s" with receiver_value_id=%s in %s::%s. ' .
                    'Chain trace so far: %s',
                    $callKind,
                    $callName,
                    $currentValueId,
                    $this->class,
                    $this->method,
                    $this->describeSteps($tracedSteps)
                )
            );

            $call = reset($calls);
            $tracedSteps[] = ['type' => 'call', 'data' => $call];

            // Find the result value for this call
            $resultValue = $this->data->getValueById($call['id']);

            Assert::assertNotNull(
                $resultValue,
                sprintf(
                    'Could not find result value for call id=%s (%s to %s)',
                    $call['id'],
                    $callKind,
                    $callName
                )
            );

            Assert::assertEquals(
                'result',
                $resultValue['kind'] ?? '',
                sprintf(
                    'Expected result value for call id=%s, got kind=%s',
                    $call['id'],
                    $resultValue['kind'] ?? 'null'
                )
            );

            $tracedSteps[] = ['type' => 'value', 'data' => $resultValue];
            $currentValueId = $resultValue['id'];
        }

        $finalValue = end($tracedSteps)['data'];

        return new ChainVerificationResult(
            steps: $tracedSteps,
            rootValue: $startValue,
            finalValue: $finalValue,
            stepCount: count($this->steps),
            finalType: $finalValue['type'] ?? null
        );
    }

    /**
     * @return array<string, mixed>|null
     */
    private function findStartingValue(string $scopePattern): ?array
    {
        // Handle $this - look for parameter or special handling
        if ($this->startingVar === '$this') {
            // $this is typically not an explicit value in calls.json
            // Find the first call in the scope and work backwards
            return $this->findThisValue($scopePattern);
        }

        // Check if it's a parameter
        $varNameWithoutDollar = substr($this->startingVar, 1);
        $paramPattern = $scopePattern . '.($' . $varNameWithoutDollar . ')';

        $values = (new ValueQuery($this->data))
            ->kind('parameter')
            ->symbolContains($paramPattern)
            ->all();

        if (!empty($values)) {
            return $values[0];
        }

        // Check if it's a local variable
        $localPattern = $scopePattern . '.local$' . $varNameWithoutDollar;

        $values = (new ValueQuery($this->data))
            ->kind('local')
            ->symbolContains($localPattern)
            ->all();

        if (!empty($values)) {
            return $values[0];
        }

        return null;
    }

    /**
     * Find a synthetic "this" value for chains starting with $this.
     *
     * @return array<string, mixed>|null
     */
    private function findThisValue(string $scopePattern): ?array
    {
        // Look for any call in this scope that accesses a property on $this
        // The receiver_value_id should point to a parameter named $this or
        // we create a synthetic representation

        // First, try to find calls that access promoted properties (these would have
        // receiver pointing to something)
        $calls = (new CallQuery($this->data))
            ->callerContains($scopePattern)
            ->kind('access')
            ->hasReceiver()
            ->all();

        if (!empty($calls)) {
            // Get the receiver value of the first access call
            $firstCall = $calls[0];
            $receiverId = $firstCall['receiver_value_id'] ?? null;

            if ($receiverId) {
                $receiverValue = $this->data->getValueById($receiverId);
                if ($receiverValue) {
                    return $receiverValue;
                }
            }
        }

        // If no explicit $this value, create a synthetic one for testing
        return [
            'id' => '__synthetic_this__',
            'kind' => 'parameter',
            'symbol' => $scopePattern . '.($this)',
            'type' => null,
        ];
    }

    /**
     * @param list<array{type: string, data: array<string, mixed>}> $steps
     */
    private function describeSteps(array $steps): string
    {
        $descriptions = [];
        foreach ($steps as $step) {
            $data = $step['data'];
            if ($step['type'] === 'value') {
                $descriptions[] = sprintf(
                    'value(%s, id=%s)',
                    $data['kind'] ?? '?',
                    $data['id'] ?? '?'
                );
            } else {
                $descriptions[] = sprintf(
                    'call(%s, callee=%s)',
                    $data['kind'] ?? '?',
                    $data['callee'] ?? '?'
                );
            }
        }
        return implode(' -> ', $descriptions);
    }

    private function validateConfiguration(): void
    {
        if ($this->class === null || $this->method === null || $this->startingVar === null) {
            throw new \InvalidArgumentException(
                'Must call startingFrom() before verify()'
            );
        }

        if (empty($this->steps)) {
            throw new \InvalidArgumentException(
                'Must add at least one step with throughAccess() or throughMethod()'
            );
        }
    }
}
