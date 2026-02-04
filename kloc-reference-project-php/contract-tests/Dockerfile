FROM php:8.4-cli

# Install dependencies
RUN apt-get update && apt-get install -y \
    git \
    unzip \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install scip CLI for SCIP to JSON conversion
# Detect architecture and download appropriate binary
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        curl -L https://github.com/sourcegraph/scip/releases/download/v0.5.2/scip-linux-amd64.tar.gz | tar xz; \
    elif [ "$ARCH" = "arm64" ]; then \
        curl -L https://github.com/sourcegraph/scip/releases/download/v0.5.2/scip-linux-arm64.tar.gz | tar xz; \
    else \
        echo "Unsupported architecture: $ARCH" && exit 1; \
    fi && \
    mv scip /usr/local/bin/

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
