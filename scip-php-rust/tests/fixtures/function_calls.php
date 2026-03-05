<?php

namespace App\Helpers;

function formatName(string $first, string $last): string {
    return trim($first) . ' ' . trim($last);
}

function greetUser(string $name): string {
    $formatted = formatName($name, '');
    return 'Hello, ' . $formatted;
}
