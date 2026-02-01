#!/usr/bin/env php
<?php

declare(strict_types=1);

/**
 * Generate documentation with live test execution status.
 *
 * Usage:
 *   php bin/generate-docs.php                    # Run tests + generate markdown
 *   php bin/generate-docs.php --format=json      # Run tests + generate JSON
 *   php bin/generate-docs.php --format=csv       # Run tests + generate CSV
 *   php bin/generate-docs.php --skip-tests       # Use cached results if available
 */

require_once __DIR__ . '/../vendor/autoload.php';

use ContractTests\Attribute\ContractTest;

// Parse arguments
$format = 'markdown';
$skipTests = false;
$outputFile = null;

foreach ($argv as $arg) {
    if (str_starts_with($arg, '--format=')) {
        $format = substr($arg, 9);
    }
    if ($arg === '--skip-tests') {
        $skipTests = true;
    }
    if (str_starts_with($arg, '--output=')) {
        $outputFile = substr($arg, 9);
    }
}

$junitFile = __DIR__ . '/../output/junit.xml';
$jsonMetaFile = __DIR__ . '/../output/test-metadata.json';

// Step 1: Run PHPUnit with JUnit output
if (!$skipTests || !file_exists($junitFile)) {
    fwrite(STDERR, "Running PHPUnit tests...\n");
    $phpunitCmd = 'vendor/bin/phpunit --log-junit ' . escapeshellarg($junitFile) . ' 2>&1';
    exec($phpunitCmd, $output, $exitCode);
    fwrite(STDERR, "PHPUnit completed with exit code: {$exitCode}\n\n");
}

// Step 2: Parse JUnit results
$testResults = parseJunitResults($junitFile);

// Step 3: Extract test metadata from attributes
$testMetadata = extractTestMetadata();

// Step 4: Merge results with metadata
$tests = mergeResultsWithMetadata($testMetadata, $testResults);

// Step 5: Output in requested format
$output = match ($format) {
    'json' => outputJson($tests),
    'csv' => outputCsv($tests),
    default => outputMarkdown($tests),
};

if ($outputFile) {
    file_put_contents($outputFile, $output);
    fwrite(STDERR, "Documentation written to: {$outputFile}\n");
} else {
    echo $output;
}

// Also save metadata for caching
file_put_contents($jsonMetaFile, json_encode($tests, JSON_PRETTY_PRINT));

// ============================================================================
// Functions
// ============================================================================

function parseJunitResults(string $junitFile): array
{
    if (!file_exists($junitFile)) {
        return [];
    }

    $xml = simplexml_load_file($junitFile);
    $results = [];

    // Recursively find all testcases in nested testsuites
    parseTestsuiteRecursive($xml, $results);

    return $results;
}

function parseTestsuiteRecursive(SimpleXMLElement $element, array &$results): void
{
    // Process testcases at this level
    foreach ($element->testcase as $testcase) {
        $class = (string) $testcase['class'];
        $method = (string) $testcase['name'];
        $key = $class . '::' . $method;

        $status = 'passed';
        $message = '';

        if (isset($testcase->failure)) {
            $status = 'failed';
            $message = (string) $testcase->failure;
        } elseif (isset($testcase->error)) {
            $status = 'error';
            $message = (string) $testcase->error;
        } elseif (isset($testcase->skipped)) {
            $status = 'skipped';
            $message = (string) $testcase->skipped;
        }

        $results[$key] = [
            'status' => $status,
            'time' => (float) $testcase['time'],
            'message' => trim($message),
        ];
    }

    // Recurse into nested testsuites
    foreach ($element->testsuite as $suite) {
        parseTestsuiteRecursive($suite, $results);
    }
}

function extractTestMetadata(): array
{
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

            $key = $className . '::' . $method->getName();
            $attributes = $method->getAttributes(ContractTest::class);

            // Generate codeRef dynamically from reflection
            $dynamicCodeRef = $className . '::' . $method->getName();

            if (empty($attributes)) {
                $tests[$key] = [
                    'name' => humanize($method->getName()),
                    'description' => '',
                    'codeRef' => $dynamicCodeRef,
                    'category' => getCategoryFromClass($className),
                    'declaredStatus' => 'active',
                ];
            } else {
                $attr = $attributes[0]->newInstance();
                $tests[$key] = [
                    'name' => $attr->name,
                    'description' => $attr->description,
                    'codeRef' => $dynamicCodeRef,
                    'category' => $attr->category ?: getCategoryFromClass($className),
                    'declaredStatus' => $attr->status,
                ];
            }
        }
    }

    return $tests;
}

function mergeResultsWithMetadata(array $metadata, array $results): array
{
    $tests = [];

    foreach ($metadata as $method => $meta) {
        $result = $results[$method] ?? ['status' => 'not_run', 'time' => 0, 'message' => ''];

        $tests[] = [
            'method' => $method,
            'name' => $meta['name'],
            'description' => $meta['description'],
            'codeRef' => $meta['codeRef'],
            'category' => $meta['category'],
            'declaredStatus' => $meta['declaredStatus'],
            'executionStatus' => $result['status'],
            'executionTime' => $result['time'],
            'message' => $result['message'],
        ];
    }

    // Sort by category then name
    usort($tests, fn($a, $b) =>
        $a['category'] <=> $b['category'] ?: $a['name'] <=> $b['name']
    );

    return $tests;
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

function getStatusEmoji(string $status): string
{
    return match ($status) {
        'passed' => '✅',
        'failed' => '❌',
        'error' => '💥',
        'skipped' => '⏭️',
        'not_run' => '⚪',
        default => '❓',
    };
}

function outputMarkdown(array $tests): string
{
    $out = "# Contract Tests Documentation\n\n";
    $out .= "Generated: " . date('Y-m-d H:i:s') . "\n\n";

    // Summary
    $summary = ['passed' => 0, 'failed' => 0, 'skipped' => 0, 'error' => 0];
    foreach ($tests as $test) {
        $status = $test['executionStatus'];
        if (isset($summary[$status])) {
            $summary[$status]++;
        }
    }

    $out .= "## Summary\n\n";
    $out .= "| Status | Count |\n";
    $out .= "|--------|-------|\n";
    $out .= "| ✅ Passed | {$summary['passed']} |\n";
    $out .= "| ❌ Failed | {$summary['failed']} |\n";
    $out .= "| ⏭️ Skipped | {$summary['skipped']} |\n";
    $out .= "| 💥 Error | {$summary['error']} |\n";
    $out .= "| **Total** | **" . count($tests) . "** |\n\n";

    $currentCategory = '';

    foreach ($tests as $test) {
        if ($test['category'] !== $currentCategory) {
            $currentCategory = $test['category'];
            $out .= "\n## " . ucfirst($currentCategory) . " Tests\n\n";
            $out .= "| Status | Test Name | Description | Code Ref |\n";
            $out .= "|--------|-----------|-------------|----------|\n";
        }

        $emoji = getStatusEmoji($test['executionStatus']);
        $codeRef = $test['codeRef'] ?: '-';

        $out .= "| {$emoji} | {$test['name']} | {$test['description']} | `{$codeRef}` |\n";
    }

    // Failed tests details
    $failed = array_filter($tests, fn($t) => $t['executionStatus'] === 'failed');
    if (!empty($failed)) {
        $out .= "\n## Failed Tests Details\n\n";
        foreach ($failed as $test) {
            $out .= "### ❌ {$test['name']}\n\n";
            $out .= "**Method**: `{$test['method']}`\n\n";
            if ($test['message']) {
                $out .= "**Error**:\n```\n" . substr($test['message'], 0, 500) . "\n```\n\n";
            }
        }
    }

    return $out;
}

function outputJson(array $tests): string
{
    $summary = ['passed' => 0, 'failed' => 0, 'skipped' => 0, 'error' => 0, 'total' => count($tests)];
    foreach ($tests as $test) {
        $status = $test['executionStatus'];
        if (isset($summary[$status])) {
            $summary[$status]++;
        }
    }

    return json_encode([
        'generated' => date('c'),
        'summary' => $summary,
        'tests' => $tests,
    ], JSON_PRETTY_PRINT) . "\n";
}

function outputCsv(array $tests): string
{
    $out = fopen('php://temp', 'r+');
    fputcsv($out, ['Status', 'Test Name', 'Description', 'Method', 'Code Ref', 'Category', 'Time (s)', 'Message']);

    foreach ($tests as $test) {
        fputcsv($out, [
            $test['executionStatus'],
            $test['name'],
            $test['description'],
            $test['method'],
            $test['codeRef'],
            $test['category'],
            number_format($test['executionTime'], 4),
            substr($test['message'], 0, 200),
        ]);
    }

    rewind($out);
    $csv = stream_get_contents($out);
    fclose($out);

    return $csv;
}
