<?php

declare(strict_types=1);

namespace App\Component;

interface EmailSenderInterface
{
    public function send(string $to, string $subject, string $body): void;
}
