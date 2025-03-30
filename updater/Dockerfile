# Use the Rust image with all required tools
FROM rust:1.81

# Install necessary system dependencies
RUN apt-get update && \
    apt-get install -y \
    jq \
    git \
    nano \
    imagemagick \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set default working directory
WORKDIR /updater

# Copy only Cargo files to leverage Docker's layer caching for dependencies
COPY Cargo.toml Cargo.lock ./

# Pre-build dependencies to cache them
RUN cargo build --release || true

# Copy the rest of the application code
COPY . .

# Run the app with live reloading
CMD ["cargo", "run"]

