<?php

declare(strict_types=1);

/**
 * Contract Tests Configuration
 *
 * Environment variables take precedence over default values.
 */
return [
    // Path to scip-php binary (relative to contract-tests/ or absolute)
    'scip_binary' => getenv('SCIP_PHP_BINARY') ?: '../../scip-php/build/scip-php',

    // Path to reference project root (relative to contract-tests/)
    'project_root' => getenv('PROJECT_ROOT') ?: '../',

    // Output directory for generated index (relative to contract-tests/)
    'output_dir' => getenv('OUTPUT_DIR') ?: './output',

    // Generated files
    'calls_json' => 'calls.json',
    'index_scip' => 'index.scip',
    'index_kloc' => 'index.kloc',
];
