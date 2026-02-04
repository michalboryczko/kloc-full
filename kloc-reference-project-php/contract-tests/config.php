<?php

declare(strict_types=1);

/**
 * Contract Tests Configuration
 *
 * NOTE: Index generation is handled by bin/run.sh using scip-php Docker image.
 * Tests only read the pre-generated calls.json file.
 */
return [
    // Output directory for generated index (relative to contract-tests/)
    'output_dir' => getenv('OUTPUT_DIR') ?: './output',

    // Generated files
    'calls_json' => 'calls.json',
    'index_scip' => 'index.scip',
    'scip_json' => 'index.scip.json',
    'index_kloc' => 'index.kloc',
];
