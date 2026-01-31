<?php

declare(strict_types=1);

namespace ContractTests\Setup;

use RuntimeException;

/**
 * Generates index files by running the scip-php binary.
 */
final class IndexGenerator
{
    private string $scipBinary;
    private string $projectRoot;
    private string $outputDir;
    private string $callsJsonFile;

    /**
     * @param array{
     *     scip_binary: string,
     *     project_root: string,
     *     output_dir: string,
     *     calls_json: string
     * } $config
     */
    public function __construct(array $config)
    {
        $this->scipBinary = $config['scip_binary'];
        $this->projectRoot = $config['project_root'];
        $this->outputDir = $config['output_dir'];
        $this->callsJsonFile = $config['calls_json'];
    }

    /**
     * Generate index files from the project source.
     *
     * If SKIP_INDEX_GENERATION env var is set, or calls.json already exists
     * and FORCE_INDEX_GENERATION is not set, skip generation.
     *
     * @throws RuntimeException if binary not found or execution fails
     */
    public function generate(): void
    {
        $callsJsonPath = $this->getCallsJsonPath();

        // Skip if env var set
        if (getenv('SKIP_INDEX_GENERATION')) {
            if (!file_exists($callsJsonPath)) {
                throw new RuntimeException(
                    sprintf(
                        'SKIP_INDEX_GENERATION is set but calls.json not found at: %s',
                        $callsJsonPath
                    )
                );
            }
            echo sprintf("Skipping index generation (SKIP_INDEX_GENERATION). Using: %s\n", $callsJsonPath);
            return;
        }

        // Reuse existing if available and not forcing regeneration
        if (file_exists($callsJsonPath) && !getenv('FORCE_INDEX_GENERATION')) {
            echo sprintf("Using existing index: %s (set FORCE_INDEX_GENERATION=1 to regenerate)\n", $callsJsonPath);
            return;
        }

        $this->ensureOutputDirectory();
        $this->validateBinary();
        $this->runScipPhp();
    }

    /**
     * Get the path to the generated calls.json file.
     */
    public function getCallsJsonPath(): string
    {
        return $this->outputDir . '/' . $this->callsJsonFile;
    }

    private function ensureOutputDirectory(): void
    {
        if (!is_dir($this->outputDir)) {
            if (!mkdir($this->outputDir, 0755, true)) {
                throw new RuntimeException(
                    sprintf('Failed to create output directory: %s', $this->outputDir)
                );
            }
        }
    }

    private function validateBinary(): void
    {
        if (!file_exists($this->scipBinary)) {
            throw new RuntimeException(
                sprintf(
                    'scip-php binary not found at: %s. ' .
                    'Build it with: cd scip-php && make build',
                    $this->scipBinary
                )
            );
        }

        if (!is_executable($this->scipBinary)) {
            throw new RuntimeException(
                sprintf(
                    'scip-php binary is not executable: %s. ' .
                    'Run: chmod +x %s',
                    $this->scipBinary,
                    $this->scipBinary
                )
            );
        }
    }

    private function runScipPhp(): void
    {
        // scip-php outputs files to the project directory by default
        // We need to run from the project directory and then move files
        $projectDir = $this->projectRoot;
        $cwd = getcwd();

        // Change to project directory for scip-php
        chdir($projectDir);

        $command = sprintf(
            '%s -d %s 2>&1',
            escapeshellarg($this->scipBinary),
            escapeshellarg($projectDir)
        );

        $output = [];
        $returnCode = 0;
        exec($command, $output, $returnCode);

        // Restore original directory
        chdir($cwd);

        if ($returnCode !== 0) {
            throw new RuntimeException(
                sprintf(
                    "scip-php execution failed with code %d.\nCommand: %s\nOutput:\n%s",
                    $returnCode,
                    $command,
                    implode("\n", $output)
                )
            );
        }

        // scip-php outputs calls.json to project directory - move it to output directory
        $sourceCallsJson = $projectDir . '/calls.json';
        $targetCallsJson = $this->getCallsJsonPath();

        if (!file_exists($sourceCallsJson)) {
            throw new RuntimeException(
                sprintf(
                    "scip-php completed but calls.json was not generated at: %s\nCommand output:\n%s",
                    $sourceCallsJson,
                    implode("\n", $output)
                )
            );
        }

        // Move to output directory
        if ($sourceCallsJson !== $targetCallsJson) {
            if (!rename($sourceCallsJson, $targetCallsJson)) {
                // Try copy+delete if rename fails (cross-filesystem)
                if (!copy($sourceCallsJson, $targetCallsJson)) {
                    throw new RuntimeException(
                        sprintf('Failed to move calls.json from %s to %s', $sourceCallsJson, $targetCallsJson)
                    );
                }
                unlink($sourceCallsJson);
            }
        }

        echo sprintf("Index generated: %s\n", $targetCallsJson);
    }
}
