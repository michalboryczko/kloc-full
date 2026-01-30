<?php

declare(strict_types=1);

namespace App\Component;

/**
 * Mock email sender implementation.
 *
 * Stores sent emails in memory instead of actually sending them.
 * Useful for development and testing.
 */
final class EmailSender implements EmailSenderInterface
{
    /** @var array<array{to: string, subject: string, body: string}> */
    private static array $sentEmails = [];

    public function send(string $to, string $subject, string $body): void
    {
        // Store the email in memory (mock behavior)
        self::$sentEmails[] = [
            'to' => $to,
            'subject' => $subject,
            'body' => $body,
        ];
    }

    /**
     * Get all sent emails (for debugging/verification).
     *
     * @return array<array{to: string, subject: string, body: string}>
     */
    public static function getSentEmails(): array
    {
        return self::$sentEmails;
    }

    /**
     * Clear all sent emails (for testing).
     */
    public static function clearSentEmails(): void
    {
        self::$sentEmails = [];
    }
}
