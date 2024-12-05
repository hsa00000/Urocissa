FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef

# Define arguments for branch and commit hash
ARG BRANCH
ARG LAST_COMMIT_HASH
ARG REPO_URL=https://github.com/hsa00000/Urocissa

# Clone the repository and check out the specific commit
RUN mkdir -p /repo

RUN git clone --branch ${BRANCH} ${REPO_URL} /repo

WORKDIR /repo
RUN git checkout ${LAST_COMMIT_HASH}

# Copy local config files into the cloned repository
# Ensure these files are in your Docker build context
# relative to the Dockerfile.
COPY gallery-frontend/config.ts ./gallery-frontend/config.ts
COPY gallery-backend/.env ./gallery-backend/.env
COPY gallery-backend/Rocket.toml ./gallery-backend/Rocket.toml

FROM chef AS planner
WORKDIR /repo/gallery-backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /repo/gallery-backend/recipe.json /repo/gallery-backend/recipe.json
WORKDIR /repo/gallery-backend
RUN cargo chef cook --release --recipe-path recipe.json

RUN git init /repo && \
    cd /repo && \
    git fetch origin ${BRANCH} && \
    git reset --hard origin/${BRANCH} && \
    git clean -df --exclude='*' && \
    git pull origin ${BRANCH}
RUN git checkout ${LAST_COMMIT_HASH}

WORKDIR /repo/gallery-backend
# Build the Rust project (cached)
RUN cargo build --release --bin Urocissa

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime

# Install required dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    npm \
    nodejs \
    --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Move the cloned repository to the dynamic path
COPY --from=chef /repo /repo
RUN mkdir -p "${UROCISSA_PATH}" && mv /repo/* "${UROCISSA_PATH}"

COPY --from=builder /repo/gallery-backend/target/release/Urocissa ${UROCISSA_PATH}/gallery-backend

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
    echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi


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

# Build the frontend
RUN npm run build

# Print success message
RUN echo "Docker image built successfully! All required steps were executed."



# Remove all items except gallery-backend and gallery-frontend
RUN cd ${UROCISSA_PATH} && \
    ls -A | grep -v '^gallery-backend$' | grep -v '^gallery-frontend$' | xargs rm -rf

# Remove the specified files from within those directories
RUN rm -f \
    ${UROCISSA_PATH}/gallery-frontend/config.default.ts \
    ${UROCISSA_PATH}/gallery-backend/.env.default \
    ${UROCISSA_PATH}/gallery-backend/Rocket.default.toml

# Set the working directory to backend for running the application
WORKDIR ${UROCISSA_PATH}/gallery-backend

# Define the command to run the application
ENTRYPOINT [ "./Urocissa" ]