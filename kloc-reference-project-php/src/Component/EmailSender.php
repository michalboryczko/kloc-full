<?php

declare(strict_types=1);

namespace App\Component;

final class EmailSender implements EmailSenderInterface
{
    /** @var array<array{to: string, subject: string, body: string}> */
    private static array $sentEmails = [];

    public function send(string $to, string $subject, string $body): void
    {
        self::$sentEmails[] = [
            'to' => $to,
            'subject' => $subject,
            'body' => $body,
        ];
    }

    /** @return array<array{to: string, subject: string, body: string}> */
    public static function getSentEmails(): array
    {
        return self::$sentEmails;
    }

    public static function clearSentEmails(): void
    {
        self::$sentEmails = [];
    }
}
