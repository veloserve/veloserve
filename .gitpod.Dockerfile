# Ona Docker Image for VeloServe Development
FROM gitpod/workspace-full

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/gitpod/.cargo/bin:${PATH}"

# Install PHP 8.3 and extensions
RUN sudo apt-get update && sudo apt-get install -y \
    php8.3 \
    php8.3-cli \
    php8.3-mysql \
    php8.3-curl \
    php8.3-gd \
    php8.3-mbstring \
    php8.3-xml \
    php8.3-zip \
    php8.3-intl \
    php8.3-bcmath \
    php8.3-soap \
    php8.3-opcache \
    && sudo rm -rf /var/lib/apt/lists/*

# Create web directory
RUN sudo mkdir -p /var/www/html && sudo chown -R gitpod:gitpod /var/www

# Set working directory
WORKDIR /workspace/veloserve

