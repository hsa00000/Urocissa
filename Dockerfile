# Use the Alpine variant of cargo-chef as the base image for building
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef

# Define the build type as a build argument (default to "release")
ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

WORKDIR /app/gallery-backend

######################
# Planner stage
######################
FROM chef AS planner

COPY ./gallery-backend/Cargo.lock /app/gallery-backend/Cargo.lock
COPY ./gallery-backend/Cargo.toml /app/gallery-backend/Cargo.toml
COPY ./gallery-backend/src /app/gallery-backend/src

RUN cargo chef prepare --recipe-path recipe.json

######################
# Builder stage
######################
FROM chef AS builder

RUN apk add --no-cache openssl-dev openssl-libs-static

COPY --from=planner /app/gallery-backend/recipe.json /app/gallery-backend/recipe.json

# Use the build argument to determine the build mode
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo chef cook --release --recipe-path recipe.json; \
    elif [ "${BUILD_TYPE}" = "debug" ]; then \
        cargo chef cook --recipe-path recipe.json; \
    else \
        cargo chef cook --profile "${BUILD_TYPE}" --recipe-path recipe.json; \
    fi

COPY ./gallery-backend/Cargo.lock /app/gallery-backend/Cargo.lock
COPY ./gallery-backend/Cargo.toml /app/gallery-backend/Cargo.toml
COPY ./gallery-backend/src /app/gallery-backend/src

# Build the backend binary based on the build type
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo build --release --bin Urocissa; \
    elif [ "${BUILD_TYPE}" = "debug" ]; then \
        cargo build --bin Urocissa; \
    else \
        cargo build --profile "${BUILD_TYPE}" --bin Urocissa; \
    fi

RUN cp /app/gallery-backend/target/${BUILD_TYPE}/Urocissa /app/gallery-backend/Urocissa

######################
# Frontend builder stage
######################
FROM node:lts-alpine AS frontend-builder
WORKDIR /app/gallery-frontend
COPY ./gallery-frontend /app/gallery-frontend
RUN npm run build

######################
# Runtime stage
######################
FROM alpine:latest AS runtime

RUN apk add --no-cache ffmpeg

WORKDIR /app/gallery-backend

# Copy backend binary
COPY --from=builder /app/gallery-backend /app/gallery-backend
COPY --from=builder /app/gallery-backend/Urocissa /app/gallery-backend/Urocissa

# Copy frontend assets
COPY --from=frontend-builder /app/gallery-frontend/dist /app/gallery-frontend/dist
COPY --from=frontend-builder /app/gallery-frontend/public /app/gallery-frontend/public

# Add an entrypoint script that will:
# 1. Check if UROCISSA_PATH is set
# 2. Move /app/gallery-* to ${UROCISSA_PATH}/gallery-* if set
# 3. Run the Urocissa binary
WORKDIR /app

# Create the entrypoint script
RUN echo '#!/bin/sh' > /entrypoint.sh && \
    echo 'set -e' >> /entrypoint.sh && \
    echo 'if [ -z "${UROCISSA_PATH}" ]; then' >> /entrypoint.sh && \
    echo '    echo "Error: UROCISSA_PATH is not set. Terminating."' >> /entrypoint.sh && \
    echo '    exit 1' >> /entrypoint.sh && \
    echo 'else' >> /entrypoint.sh && \
    echo '    mkdir -p "${UROCISSA_PATH}"' >> /entrypoint.sh && \
    echo '    echo "Directory listing of /app/gallery-backend before moving:"' >> /entrypoint.sh && \
    echo '    ls -al /app/gallery-backend' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}/gallery-backend"' >> /entrypoint.sh && \
    echo '    mv /app/gallery-backend/* "${UROCISSA_PATH}/gallery-backend"' >> /entrypoint.sh && \
    echo '    mv /app/gallery-frontend/* "${UROCISSA_PATH}/gallery-frontend"' >> /entrypoint.sh && \
    echo '    echo "Listing ${UROCISSA_PATH} after move:"' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}"' >> /entrypoint.sh && \
    echo '    echo "Listing ${UROCISSA_PATH}/gallery-backend:"' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}/gallery-backend"' >> /entrypoint.sh && \
    echo '    echo "Listing ${UROCISSA_PATH}/gallery-frontend:"' >> /entrypoint.sh && \
    echo '    ls -al "${UROCISSA_PATH}/gallery-frontend"' >> /entrypoint.sh && \
    echo '    cd "${UROCISSA_PATH}/gallery-backend"' >> /entrypoint.sh && \
    echo 'fi' >> /entrypoint.sh && \
    echo 'echo "Attempting to run ./Urocissa"' >> /entrypoint.sh && \
    echo 'ls -al' >> /entrypoint.sh && \
    echo 'exec ./Urocissa' >> /entrypoint.sh && \
    chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
