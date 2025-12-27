FROM rust:bookworm AS builder

ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

WORKDIR /app/backend

COPY ./backend/Cargo.lock /app/backend/Cargo.lock
COPY ./backend/Cargo.toml /app/backend/Cargo.toml
COPY ./backend/src /app/backend/src

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build the backend binary based on the build type
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo build --release --bin urocissa; \
    elif [ "${BUILD_TYPE}" = "debug" ]; then \
        cargo build --bin urocissa; \
    else \
        cargo build --profile "${BUILD_TYPE}" --bin urocissa; \
    fi

RUN cp /app/backend/target/${BUILD_TYPE}/urocissa /app/backend/urocissa

######################
# Frontend builder stage
######################
FROM node:lts AS frontend-builder
WORKDIR /app/frontend
COPY ./frontend /app/frontend
RUN npm ci && npm run build

######################
# Runtime stage
######################
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/backend

# Copy backend binary
COPY --from=builder /app/backend/urocissa /app/backend/urocissa

# Copy frontend assets
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
COPY --from=frontend-builder /app/frontend/public /app/frontend/public

# Add an entrypoint script that will:
# 1. Check if UROCISSA_PATH is set
# 2. Move /app/* to ${UROCISSA_PATH}/* if set
# 3. Run the urocissa binary
WORKDIR /app

# Create the entrypoint script
RUN echo '#!/bin/sh' > /entrypoint.sh && \
    echo 'set -e' >> /entrypoint.sh && \
    echo 'if [ -z "${UROCISSA_PATH}" ]; then' >> /entrypoint.sh && \
    echo '    echo "Error: UROCISSA_PATH is not set. Terminating."' >> /entrypoint.sh && \
    echo '    exit 1' >> /entrypoint.sh && \
    echo 'else' >> /entrypoint.sh && \
    echo '    mkdir -p "${UROCISSA_PATH}/backend"' >> /entrypoint.sh && \
    echo '    mkdir -p "${UROCISSA_PATH}/frontend"' >> /entrypoint.sh && \
    echo '    mv /app/backend/* "${UROCISSA_PATH}/backend"' >> /entrypoint.sh && \
    echo '    mv /app/frontend/* "${UROCISSA_PATH}/frontend"' >> /entrypoint.sh && \
    echo '    echo "Listing ${UROCISSA_PATH}/backend:"' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}/backend"' >> /entrypoint.sh && \
    echo '    echo "Listing ${UROCISSA_PATH}/frontend:"' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}/frontend"' >> /entrypoint.sh && \
    echo '    cd "${UROCISSA_PATH}/backend"' >> /entrypoint.sh && \
    echo 'fi' >> /entrypoint.sh && \
    echo 'echo "Attempting to run ./urocissa"' >> /entrypoint.sh && \
    echo 'exec ./urocissa' >> /entrypoint.sh && \
    chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]