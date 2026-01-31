#!/usr/bin/env php
<?php

declare(strict_types=1);

/**
 * Generate documentation table from ContractTest attributes.
 *
 * Usage: php bin/generate-docs.php [--format=markdown|csv]
 */

require_once __DIR__ . '/../vendor/autoload.php';

use ContractTests\Attribute\ContractTest;

$format = 'markdown';
foreach ($argv as $arg) {
    if (str_starts_with($arg, '--format=')) {
        $format = substr($arg, 9);
    }
}

// Find all test files
$testDir = __DIR__ . '/../tests';
$testFiles = new RecursiveIteratorIterator(
    new RecursiveDirectoryIterator($testDir)
);

$tests = [];

foreach ($testFiles as $file) {
    if (!$file->isFile() || $file->getExtension() !== 'php') {
        continue;
    }

    $content = file_get_contents($file->getPathname());

    // Extract namespace and class
    if (!preg_match('/namespace\s+([^;]+);/', $content, $nsMatch)) {
        continue;
    }
    if (!preg_match('/class\s+(\w+)/', $content, $classMatch)) {
        continue;
    }

    $className = $nsMatch[1] . '\\' . $classMatch[1];

    if (!class_exists($className)) {
        continue;
    }

    $reflection = new ReflectionClass($className);

    foreach ($reflection->getMethods(ReflectionMethod::IS_PUBLIC) as $method) {
        if (!str_starts_with($method->getName(), 'test')) {
            continue;
        }

        $attributes = $method->getAttributes(ContractTest::class);

        if (empty($attributes)) {
            // No attribute, create basic entry from method name
            $tests[] = [
                'name' => humanize($method->getName()),
                'description' => '',
                'method' => $className . '::' . $method->getName(),
                'codeRef' => '',
                'category' => getCategoryFromClass($className),
                'status' => 'active',
            ];
        } else {
            $attr = $attributes[0]->newInstance();
            $tests[] = [
                'name' => $attr->name,
                'description' => $attr->description,
                'method' => $className . '::' . $method->getName(),
                'codeRef' => $attr->codeRef,
                'category' => $attr->category ?: getCategoryFromClass($className),
                'status' => $attr->status,
            ];
        }
    }
}

// Sort by category then name
usort($tests, fn($a, $b) =>
    $a['category'] <=> $b['category'] ?: $a['name'] <=> $b['name']
);

// Output
if ($format === 'csv') {
    outputCsv($tests);
} else {
    outputMarkdown($tests);
}

function humanize(string $methodName): string
{
    $name = preg_replace('/^test/', '', $methodName);
    $name = preg_replace('/([A-Z])/', ' $1', $name);
    return trim($name);
}

function getCategoryFromClass(string $className): string
{
    if (str_contains($className, 'Smoke')) return 'smoke';
    if (str_contains($className, 'Integrity')) return 'integrity';
    if (str_contains($className, 'Reference')) return 'reference';
    if (str_contains($className, 'Chain')) return 'chain';
    if (str_contains($className, 'Argument')) return 'argument';
    return 'other';
}

function outputMarkdown(array $tests): void
{
    echo "# Contract Tests Documentation\n\n";
    echo "Generated: " . date('Y-m-d H:i:s') . "\n\n";

    $currentCategory = '';

    foreach ($tests as $test) {
        if ($test['category'] !== $currentCategory) {
            $currentCategory = $test['category'];
            echo "\n## " . ucfirst($currentCategory) . " Tests\n\n";
            echo "| Test Name | Description | Method | Status |\n";
            echo "|-----------|-------------|--------|--------|\n";
        }

        $status = match($test['status']) {
            'active' => '✅ Active',
            'skipped' => '⏭️ Skipped',
            'pending' => '⏳ Pending',
            default => $test['status'],
        };

        echo "| {$test['name']} | {$test['description']} | `{$test['method']}` | {$status} |\n";
    }

    echo "\n---\n";
    echo "Total: " . count($tests) . " tests\n";
}

function outputCsv(array $tests): void
{
    $out = fopen('php://stdout', 'w');
    fputcsv($out, ['Test Name', 'Description', 'Method', 'Code Reference', 'Category', 'Status']);

    foreach ($tests as $test) {
        fputcsv($out, [
            $test['name'],
            $test['description'],
            $test['method'],
            $test['codeRef'],
            $test['category'],
            $test['status'],
        ]);
    }

    fclose($out);
}
