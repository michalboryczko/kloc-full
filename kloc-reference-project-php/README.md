# Kloc Reference Project PHP

A reference Symfony 7.2 application demonstrating modern PHP 8.4 patterns for validating scip-php code analysis capabilities.

## Overview

This project serves as a test fixture for the Kloc PHP analyzer (scip-php), demonstrating:

- Symfony controller patterns with `#[Route]` attributes
- Request/Response DTOs with constructor property promotion
- Symfony Messenger for async message handling
- Service layer architecture
- Component-based design (EmailSender)
- Repository pattern with in-memory storage

## Requirements

- Docker and Docker Compose
- OR PHP 8.4+ with Composer

## Quick Start with Docker

```bash
# Build and start the container
docker-compose up -d --build

# Install dependencies
docker-compose exec php composer install

# The API is available at http://localhost:8000
```

## Quick Start without Docker

```bash
# Install dependencies
composer install

# Start the development server
php -S localhost:8000 -t public
```

## Project Structure

```
src/
  Component/           # Reusable components (EmailSender)
  Entity/              # Domain entities (Order)
  Repository/          # Data access (OrderRepository)
  Service/             # Business logic (OrderService, NotificationService)
  Ui/
    Rest/
      Controller/      # REST API controllers
      Request/         # Request DTOs
      Response/        # Response DTOs
    Messenger/
      Handler/         # Message handlers
      Message/         # Message DTOs
config/
  bundles.php          # Symfony bundles
  routes.yaml          # Route configuration
  services.yaml        # Service configuration
  packages/            # Package-specific configuration
public/
  index.php            # Symfony front controller
docker/
  Dockerfile           # PHP 8.4 image definition
```

## API Endpoints

### List Orders

```bash
GET /api/orders
```

Response:
```json
{
  "orders": [
    {
      "id": 1,
      "customerEmail": "john.doe@example.com",
      "productId": "PROD-001",
      "quantity": 2,
      "status": "pending",
      "createdAt": "2024-01-15T10:30:00+00:00"
    }
  ],
  "total": 3
}
```

### Get Order

```bash
GET /api/orders/{id}
```

Response:
```json
{
  "id": 1,
  "customerEmail": "john.doe@example.com",
  "productId": "PROD-001",
  "quantity": 2,
  "status": "pending",
  "createdAt": "2024-01-15T10:30:00+00:00"
}
```

### Create Order

```bash
POST /api/orders
Content-Type: application/json

{
  "customerEmail": "customer@example.com",
  "productId": "PROD-100",
  "quantity": 3
}
```

Response:
```json
{
  "id": 4,
  "customerEmail": "customer@example.com",
  "productId": "PROD-100",
  "quantity": 3,
  "status": "pending",
  "createdAt": "2024-01-30T12:00:00+00:00"
}
```

## Architecture

### Flow: POST /api/orders

1. `OrderController::create()` receives the request
2. `#[MapRequestPayload]` deserializes JSON to `CreateOrderRequest` DTO
3. `OrderService::createOrder()` is called:
   - Creates `Order` entity
   - Saves via `OrderRepository`
   - Sends email via `EmailSender` component
4. Controller dispatches `OrderCreatedMessage` to Messenger
5. `OrderCreatedHandler::__invoke()` processes the message:
   - Calls `NotificationService::notifyOrderCreated()`
   - Sends follow-up email via `EmailSender` component

### Key Patterns

- **Readonly DTOs**: All request/response/message classes are `final readonly`
- **Constructor Promotion**: All DTOs use PHP 8 constructor property promotion
- **Symfony Attributes**: Routes, message handlers, and validation use attributes
- **Dependency Injection**: All services use constructor injection

## scip-php Analysis

This project is designed to be analyzed by scip-php:

```bash
# From the scip-php directory
cd scip-php
./build/build.sh  # Build Docker image (once)
./bin/scip-php.sh -d ../kloc-reference-project-php -o /tmp/output
```

This generates:
- `index.scip` - SCIP index file
- `calls.json` - Call graph data (with v4 calls tracking)
- `index.kloc` - Combined Kloc archive
