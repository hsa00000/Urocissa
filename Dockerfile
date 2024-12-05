# Stage 1: Chef - Install dependencies and clone the repository
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

# Install necessary dependencies
RUN apt update && apt install -y \
    ffmpeg \
    npm \
    pkg-config \
    nodejs \
    git \
    && apt clean

# Define arguments for branch and commit hash
ARG BRANCH
ARG LAST_COMMIT_HASH
ARG REPO_URL=https://github.com/hsa00000/Urocissa

# Clone the repository and check out the specific commit
RUN mkdir -p /repo
RUN git clone --branch ${BRANCH} ${REPO_URL} /repo

WORKDIR /repo
RUN git checkout ${LAST_COMMIT_HASH}

# Stage 2: Planner - Prepare cargo chef
FROM chef AS planner
WORKDIR /repo/gallery-backend
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Cook dependencies, build backend and frontend
FROM chef AS builder

# Define a stable build directory for Rust cache
ENV CARGO_TARGET_DIR=/usr/local/cargo-target

# Copy recipe and cook dependencies
COPY --from=planner /repo/gallery-backend/recipe.json /repo/gallery-backend/recipe.json
WORKDIR /repo/gallery-backend
RUN cargo chef cook --recipe-path recipe.json

# Build the Rust backend
RUN cargo build --release

# Build the frontend
WORKDIR /repo/gallery-frontend
# Copy frontend files
COPY /repo/gallery-frontend/package.json /repo/gallery-frontend/package-lock.json ./
# Install frontend dependencies
RUN npm install
# Copy the rest of the frontend source code
COPY /repo/gallery-frontend/. .
# Build the frontend
RUN npm run build

# Stage 4: Runtime - Setup the runtime environment
FROM lukemathwalker/cargo-chef:latest-rust-1 AS runtime

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Create the dynamic path
RUN mkdir -p "${UROCISSA_PATH}"

# Copy the backend binary
COPY --from=builder /usr/local/cargo-target/release/Urocissa ${UROCISSA_PATH}/gallery-backend

# Copy necessary configuration files
COPY --from=chef /repo/gallery-backend/.env.default ${UROCISSA_PATH}/gallery-backend/.env
COPY --from=chef /repo/gallery-backend/Rocket.default.toml ${UROCISSA_PATH}/gallery-backend/Rocket.toml

# Copy the built frontend assets
COPY --from=builder /repo/gallery-frontend/dist ${UROCISSA_PATH}/gallery-frontend/dist

# Set the working directory to backend for running the application
WORKDIR ${UROCISSA_PATH}/gallery-backend

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
    echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi

# Print success message
RUN echo "Docker image built successfully! All required steps were executed."

# Define the command to run the application
CMD ["./Urocissa"]
