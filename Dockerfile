# Stage 1: Base image with cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Stage 2: Dependency planner
FROM chef AS planner

# Define arguments for branch and commit hash
ARG BRANCH=optimize/use-cargo-chef
ARG REPO_URL=https://github.com/hsa00000/Urocissa

# Fetch the latest commit hash of the specified branch
RUN LATEST_COMMIT=$(git ls-remote ${REPO_URL} ${BRANCH} | awk '{print $1}') && \
    echo "Latest commit is $LATEST_COMMIT" && \
    echo $LATEST_COMMIT > /tmp/latest_commit_hash

# Clone the repository into a stable path using the commit hash
RUN mkdir -p /repo && \
    git clone -b ${BRANCH} ${REPO_URL} /repo && \
    cd /repo && \
    git checkout $(cat /tmp/latest_commit_hash)

# Copy the repository for planning dependencies
COPY . . 
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies
FROM chef AS builder

# Copy the recipe.json from the planner stage
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (cached)
RUN cargo chef cook --release --recipe-path recipe.json

# Define arguments for dynamic paths
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Clone the repository again to build the project
COPY --from=planner /repo /repo
WORKDIR /repo/gallery-backend

# Build the Rust project with caching enabled
RUN cargo build --profile dev-release

# Stage 4: Runtime stage
FROM debian:bookworm-slim AS runtime

# Install necessary runtime dependencies
RUN apt update && apt install -y \
    ffmpeg \
    npm \
    nodejs \
    && apt clean

# Copy the built application from the builder stage
COPY --from=builder /repo /app
WORKDIR /app/gallery-backend

# Ensure required backend files exist and copy defaults if missing
RUN if [ ! -f ".env" ]; then \
        cp .env.default .env; \
    fi && \
    if [ ! -f "Rocket.toml" ]; then \
        cp Rocket.default.toml Rocket.toml; \
    fi

# Set the working directory to backend for running the application
WORKDIR /app/gallery-backend

# Define the command to run the application
CMD ["cargo", "run", "--profile", "dev-release"]
