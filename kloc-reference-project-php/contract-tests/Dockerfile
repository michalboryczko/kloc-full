FROM php:8.4-cli

# Install dependencies
RUN apt-get update && apt-get install -y \
    git \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Composer
COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

# Set working directory
WORKDIR /app/contract-tests

# Copy composer files first for caching
COPY contract-tests/composer.json contract-tests/composer.lock* ./

# Install PHP dependencies
RUN composer install --no-scripts --no-autoloader || true

# Copy the rest
COPY . /app/

# Generate autoloader
RUN cd /app/contract-tests && composer dump-autoload --optimize

# Default command
CMD ["vendor/bin/phpunit"]
