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

# We do not need the Rust toolchain to run the binary!
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

SHELL [ "bash", "-c" ]

# Use ARG to allow flexibility if you want to change the Node.js version later
ARG FNM_INSTALL_VERSION=22
ENV FNM_DIR="/opt/fnm"

# Install fnm
RUN curl -fsSL https://fnm.vercel.app/install | bash -s -- --install-dir "$FNM_DIR" --skip-shell && \
    ln -s "${FNM_DIR}/fnm" /usr/bin/fnm && chmod +x /usr/bin/fnm && \
    fnm -V

RUN eval "$(fnm env --shell bash)" && \
    fnm use --install-if-missing ${FNM_INSTALL_VERSION} && \
    ln -s "${FNM_DIR}/aliases/default/bin/node" /usr/bin/node && \
    ln -s "${FNM_DIR}/aliases/default/bin/npm" /usr/bin/npm && \
    ln -s "${FNM_DIR}/aliases/default/bin/npx" /usr/bin/npx

# Verify installation
RUN node -v && npm -v

# Define a dynamic repository path
ARG UROCISSA_PATH
ENV UROCISSA_PATH=${UROCISSA_PATH}

# Ensure the target directory exists
RUN mkdir -p "${UROCISSA_PATH}/gallery-backend"

# Copy the backend build artifacts to the appropriate location
COPY --from=builder /repo/gallery-backend/target/${BUILD_TYPE}/Urocissa ${UROCISSA_PATH}/gallery-backend

# Validate if UROCISSA_PATH is set
RUN if [ -z "${UROCISSA_PATH}" ]; then \
        echo "UROCISSA_PATH is not set! Build failed." && exit 1; \
    fi

COPY ./gallery-frontend ${UROCISSA_PATH}/gallery-frontend

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
