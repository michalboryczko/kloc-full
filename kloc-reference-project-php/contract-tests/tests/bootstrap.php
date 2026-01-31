<?php

declare(strict_types=1);

/**
 * PHPUnit Bootstrap
 *
 * This file runs ONCE before all tests to:
 * 1. Load the autoloader
 * 2. Load configuration
 * 3. Generate the index using scip-php
 * 4. Define the CALLS_JSON_PATH constant for tests
 */

require_once __DIR__ . '/../vendor/autoload.php';

use ContractTests\Setup\IndexGenerator;

// Load configuration
$config = require __DIR__ . '/../config.php';

// Resolve relative paths to absolute paths (relative to contract-tests directory)
$contractTestsDir = dirname(__DIR__);
$config['scip_binary'] = resolvePath($config['scip_binary'], $contractTestsDir);
$config['project_root'] = resolvePath($config['project_root'], $contractTestsDir);
$config['output_dir'] = resolvePath($config['output_dir'], $contractTestsDir);

/**
 * Resolve a path that may be relative to a base directory.
 */
function resolvePath(string $path, string $baseDir): string
{
    if (str_starts_with($path, '/')) {
        return $path;
    }
    return realpath($baseDir . '/' . $path) ?: $baseDir . '/' . $path;
}

// Generate index ONCE before all tests
$generator = new IndexGenerator($config);
$generator->generate();

// Store calls.json path for tests
define('CALLS_JSON_PATH', $generator->getCallsJsonPath());
