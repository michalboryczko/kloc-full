<?php

namespace App\Services;

use App\Models\User;
use App\Contracts\Greetable;
use Psr\Log\LoggerInterface as Logger;

class UserService implements Greetable {
    private Logger $logger;

    public function __construct(Logger $logger) {
        $this->logger = $logger;
    }

    public function greet(): string {
        return 'Hello';
    }

    public function createUser(string $name): User {
        return new User($name, 0);
    }
}
