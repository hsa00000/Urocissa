

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

# Install necessary dependencies
RUN apt update && apt install -y \
    ffmpeg \
    npm \
    pkg-config \
    nodejs \
    && apt clean

# Define arguments for branch and commit hash
ARG BRANCH=${BRANCH}
ARG LAST_COMMIT_HASH=${LAST_COMMIT_HASH}
ARG REPO_URL=https://github.com/hsa00000/Urocissa

# Define a stable build directory for Rust cache
ENV CARGO_TARGET_DIR=/usr/local/cargo-target

# Clone the repository and check out the specific commit

RUN mkdir -p /repo
RUN git clone --branch ${BRANCH} ${REPO_URL} /repo

WORKDIR /repo

RUN git checkout ${LAST_COMMIT_HASH}

WORKDIR /repo/gallery-backend

FROM chef AS planner
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /repo/gallery-backend/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
COPY . .

# Build the Rust project (cached)
RUN cargo build 

COPY --from=builder /usr/local/cargo-target/release/urocissa ${UROCISSA_PATH}/gallery-backend

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
    echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi

# Move the cloned repository to the dynamic path
RUN mkdir -p "${UROCISSA_PATH}" && mv /repo/* "${UROCISSA_PATH}"

WORKDIR ${UROCISSA_PATH}/gallery-backend
# Ensure required backend and frontend files exist and copy defaults if missing
RUN if [ ! -f ".env" ]; then \
        cp .env.default .env; \
    fi && \
    if [ ! -f "Rocket.toml" ]; then \
        cp Rocket.default.toml Rocket.toml; \
    fi

# Switch to the frontend directory
WORKDIR ${UROCISSA_PATH}/gallery-frontend

# Copy existing config file into the container if it exists on the host
COPY ./gallery-frontend/config.ts  ${UROCISSA_PATH}/gallery-frontend/config.ts

# Build the frontend
RUN npm run build

# Print success message
RUN echo "Docker image built successfully! All required steps were executed."

# Set the working directory to backend for running the application
WORKDIR ${UROCISSA_PATH}/gallery-backend

# Define the command to run the application
CMD ["./urocissa"]
