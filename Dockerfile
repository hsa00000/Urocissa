# Use the official Rust image
FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef

# Define the build type as a build argument
ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

WORKDIR /repo

FROM chef AS planner
COPY ./gallery-backend /repo/gallery-backend

WORKDIR /repo/gallery-backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /repo/gallery-backend/recipe.json /repo/gallery-backend/recipe.json

WORKDIR /repo/gallery-backend
# Use the build argument in the chef cook step
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo chef cook --release --recipe-path recipe.json; \
    else \
        cargo chef cook --recipe-path recipe.json; \
    fi

COPY ./gallery-backend /repo/gallery-backend

WORKDIR /repo/gallery-backend
# Use the build argument in the cargo build step
RUN if [ "${BUILD_TYPE}" = "release" ]; then \
        cargo build --release --bin Urocissa; \
    else \
        cargo build --bin Urocissa; \
    fi

FROM node:22-slim AS frontend-builder

WORKDIR /repo/gallery-frontend

COPY ./gallery-frontend /repo/gallery-frontend
# Build the frontend
RUN npm run build

FROM debian:bookworm-slim AS runtime

ARG BUILD_TYPE=release
ENV BUILD_TYPE=${BUILD_TYPE}

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    xz-utils \
    curl \
    ca-certificates \
    unzip \
    --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set environment variable for the FFmpeg URL
ARG FFMPEG_BASE_URL=https://johnvansickle.com/ffmpeg/releases
ARG FFMPEG_VERSION=ffmpeg-release

# Download and install the appropriate FFmpeg binary based on architecture
RUN ARCH=$(uname -m) && \
    case "${ARCH}" in \
      x86_64)   FFMPEG_ARCH=amd64 ;; \
      i386)     FFMPEG_ARCH=i686 ;; \
      arm64)    FFMPEG_ARCH=arm64 ;; \
      armhf)    FFMPEG_ARCH=armhf ;; \
      armel)    FFMPEG_ARCH=armel ;; \
      *)        echo "Unsupported architecture: ${ARCH}" && exit 1 ;; \
    esac && \
    FFMPEG_URL="${FFMPEG_BASE_URL}/${FFMPEG_VERSION}-${FFMPEG_ARCH}-static.tar.xz" && \
    echo "Downloading FFmpeg from ${FFMPEG_URL}" && \
    curl -L "${FFMPEG_URL}" | tar -xJ -C /usr/local/bin --strip-components=1 --wildcards '*/ffmpeg' '*/ffprobe'

# Verify installation
RUN ffmpeg -version

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
        echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi

# Ensure the target directory exists
RUN mkdir -p "${UROCISSA_PATH}/gallery-backend"

# Copy the backend build artifacts to the appropriate location
COPY --from=builder /repo/gallery-backend/target/${BUILD_TYPE}/Urocissa ${UROCISSA_PATH}/gallery-backend

# Copy built frontend files from frontend-builder stage
COPY --from=frontend-builder /repo/gallery-frontend/dist ${UROCISSA_PATH}/gallery-frontend/dist
COPY --from=frontend-builder /repo/gallery-frontend/public ${UROCISSA_PATH}/gallery-frontend/public

COPY ./gallery-frontend/config.ts ${UROCISSA_PATH}/gallery-frontend
COPY ./gallery-backend/.env ${UROCISSA_PATH}/gallery-backend
COPY ./gallery-backend/Rocket.toml ${UROCISSA_PATH}/gallery-backend

# Set the working directory to backend for running the application
WORKDIR ${UROCISSA_PATH}/gallery-backend

# Print success message
RUN echo "Docker image built successfully! All required steps were executed."

# Define the command to run the application
ENTRYPOINT [ "./Urocissa" ]
