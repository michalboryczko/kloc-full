<?php

declare(strict_types=1);

namespace App\Component;

/**
 * Interface for sending emails.
 *
 * Implementations can be mocked for testing or replaced with real providers.
 */
interface EmailSenderInterface
{
    /**
     * Send an email.
     *
     * @param string $to      Recipient email address
     * @param string $subject Email subject
     * @param string $body    Email body content
     */
    public function send(string $to, string $subject, string $body): void;
}
