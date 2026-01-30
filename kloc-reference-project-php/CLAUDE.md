# Claude Instructions for kloc-reference-project-php

## Project Overview

This is a Symfony 7.2 reference application for testing scip-php code analysis. The project demonstrates modern PHP 8.4 patterns including readonly classes, constructor property promotion, and Symfony Messenger.

## Important Guidelines

### Docker Environment

Always use Docker for running the application:

```bash
# Start containers
docker-compose up -d

# Run composer commands
docker-compose exec php composer install

# Check PHP version
docker-compose exec php php -v
```

### No Tests

This project intentionally has NO tests. Do not add test files.

### Namespace

The project uses `App\` namespace. All classes in `src/` follow this pattern.

### Key Patterns

1. **DTOs are readonly**: All Request/Response/Message classes use `final readonly class`
2. **Constructor promotion**: All DTOs use constructor property promotion
3. **No database**: Use in-memory storage in OrderRepository
4. **Symfony attributes**: Use `#[Route]`, `#[MapRequestPayload]`, `#[AsMessageHandler]`

### Directory Structure

- `src/Ui/Rest/Controller/` - REST controllers
- `src/Ui/Rest/Request/` - Request DTOs
- `src/Ui/Rest/Response/` - Response DTOs
- `src/Ui/Messenger/Handler/` - Message handlers
- `src/Ui/Messenger/Message/` - Message DTOs
- `src/Service/` - Business logic
- `src/Component/` - Reusable components
- `src/Repository/` - Data access
- `src/Entity/` - Domain entities

### Validation Command

To validate the project can be analyzed by scip-php:

```bash
# From kloc repository root
./scip-php/build/scip-php kloc-reference-project-php
```

### Code Style

- Declare strict types in all files: `declare(strict_types=1);`
- Use `final` for all concrete classes
- Use `readonly` for immutable classes
- Named arguments for better readability
