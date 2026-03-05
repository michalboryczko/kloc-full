<?php

namespace App\Types;

use App\Models\User;

function withNullable(?User $user): ?string {
    return $user?->getName();
}

function withUnion(int|string $value): void {}

function withIntersection(Countable&Iterator $iter): void {}
