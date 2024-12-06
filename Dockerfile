# Use the Alpine variant of cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef

# Define the build type as a build argument
ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

WORKDIR /app/gallery-backend

FROM chef AS planner
COPY ./gallery-backend /app/gallery-backend

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install dependencies needed for building
RUN apk add --no-cache \
    openssl-dev \
    openssl-libs-static

COPY --from=planner /repo/gallery-backend/recipe.json /repo/gallery-backend/recipe.json

# Use the build argument in the chef cook step
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo chef cook --release --recipe-path recipe.json; \
    else \
        cargo chef cook --recipe-path recipe.json; \
    fi

COPY ./gallery-backend /app/gallery-backend

# Use the build argument in the cargo build step
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo build --release --bin Urocissa; \
    else \
        cargo build --bin Urocissa; \
    fi

FROM node:lts-alpine AS frontend-builder

WORKDIR /app/gallery-frontend

COPY ./gallery-frontend /app/gallery-frontend
# Build the frontend
RUN npm run build

FROM alpine:latest AS runtime

COPY --from=mwader/static-ffmpeg:latest /ffmpeg /usr/local/bin/
# COPY --from=mwader/static-ffmpeg:latest /ffprobe /usr/local/bin/

# Verify installation
RUN ffmpeg -version

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
        echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi

# Ensure the target directory exists
RUN mkdir -p "${UROCISSA_PATH}/gallery-backend"

# Copy the backend build artifacts to the appropriate location
COPY --from=builder /repo/gallery-backend/target/${BUILD_TYPE}/Urocissa ${UROCISSA_PATH}/gallery-backend

# Copy built frontend files from frontend-builder stage
COPY --from=frontend-builder /app/gallery-frontend/dist ${UROCISSA_PATH}/gallery-frontend/dist
COPY --from=frontend-builder /app/gallery-frontend/public ${UROCISSA_PATH}/gallery-frontend/public

COPY ./gallery-frontend/config.ts ${UROCISSA_PATH}/gallery-frontend
COPY ./gallery-backend/.env ${UROCISSA_PATH}/gallery-backend
COPY ./gallery-backend/Rocket.toml ${UROCISSA_PATH}/gallery-backend

# Set the working directory to backend for running the application
WORKDIR ${UROCISSA_PATH}/gallery-backend

# Print success message
RUN echo "Docker image built successfully! All required steps were executed."

# Define the command to run the application
ENTRYPOINT [ "./Urocissa" ]
