#!/bin/bash

# ============================================================
# Function Definitions
# ============================================================

show_help() {
    cat <<EOF
Usage: ./build_urocissa_docker.sh [OPTIONS]

Description:
  This script builds the Urocissa Docker image for a specified architecture and pushes it.
  It allows specifying a build type (release, debug, or custom profiles), enabling debug mode, 
  logging to a file, and disabling Docker cache.

Options:
  --help              Show this help message and exit.
  --debug             Enable debug mode to display additional information during execution.
  --log-file <file>   Specify a log file for debug output. The file will be created if it does not exist,
                      or cleared if it already exists.
  --build-type <type> Specify the build type for the Docker image. Valid values are:
                      - release (default)
                      - debug
                      - Any valid custom profile defined in Cargo.toml (e.g., dev-release)
  --no-cache          Disable Docker build cache. Forces a fresh build of all layers.
  --arch <architecture> 
                      Specify the target architecture (e.g., amd64 or arm64).

Examples:
  1. Build for amd64 with default settings (release build):
     ./build_urocissa_docker.sh --arch amd64

  2. Build for arm64 with debug configuration:
     ./build_urocissa_docker.sh --arch arm64 --build-type debug

Notes:
  - The log file specified with --log-file will be initialized at the start.
  - Debug mode outputs information to the terminal unless a log file is specified.
  - If --build-type is not specified, the default is "release".
  - The --build-type option supports custom profiles as defined in Cargo.toml.
  - The --no-cache option ensures no intermediate layers are used from previous builds.
  - The --arch option is required. Valid values are typically "amd64" and "arm64".
EOF
}

debug_log() {
    local message="$1"
    if [[ "$DEBUG" == true ]]; then
        if [[ -n "$LOG_FILE" ]]; then
            echo "$message" >>"$LOG_FILE"
        else
            echo "$message"
        fi
    fi
}

get_rust_build_profiles() {
    local cargo_toml="./gallery-backend/Cargo.toml"
    if [[ ! -f "$cargo_toml" ]]; then
        echo "Cargo.toml not found at $cargo_toml"
        return 1
    fi
    grep -Eo "^\[profile\.[a-zA-Z0-9_-]+\]" "$cargo_toml" | awk -F '.' '{print $2}' | tr -d '[]'
}

validate_build_type() {
    local build_type="$1"
    local profiles
    profiles=$(get_rust_build_profiles)
    if [[ $? -ne 0 ]]; then
        echo "Error: Unable to validate build type due to missing or invalid Cargo.toml."
        exit 1
    fi
    valid_profiles=$(echo -e "release\ndebug\n$profiles" | sort -u)
    if ! echo "$valid_profiles" | grep -qw "$build_type"; then
        echo "Error: Invalid build type '$build_type'. Valid build types are: $(echo "$valid_profiles" | tr '\n' ' ')"
        exit 1
    fi
}

validate_arch() {
    local arch="$1"
    # We can validate that arch is one of the recognized architectures
    # For this example, let's accept amd64 and arm64.
    if [[ "$arch" != "amd64" && "$arch" != "arm64" ]]; then
        echo "Error: Invalid architecture '$arch'. Valid values: amd64, arm64"
        exit 1
    fi
}

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --help)
            show_help
            exit 0
            ;;
        --debug)
            DEBUG=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --log-file)
            LOG_FILE="$2"
            >"$LOG_FILE"
            if [[ $? -ne 0 ]]; then
                echo "Error: Failed to initialize log file at $LOG_FILE"
                exit 1
            fi
            shift 2
            ;;
        --build-type)
            BUILD_TYPE="$2"
            if [[ -z "$BUILD_TYPE" ]]; then
                echo "Error: Build type is required."
                exit 1
            fi
            validate_build_type "$BUILD_TYPE"
            shift 2
            ;;
        --arch)
            ARCH="$2"
            if [[ -z "$ARCH" ]]; then
                echo "Error: Architecture is required."
                exit 1
            fi
            validate_arch "$ARCH"
            shift 2
            ;;
        *)
            echo "Error: Unknown option $1"
            exit 1
            ;;
        esac
    done
}

build_docker_image() {
    debug_log "Setting up environment for multiarch builds..."
    debug_log "Building Docker image with BUILD_TYPE=$BUILD_TYPE for ARCH=$ARCH"

    # Tag includes architecture suffix so we don't overwrite images
    IMAGE_TAG="hsa00000/urocissa:latest-$ARCH"

    DOCKER_BUILD_COMMAND="docker build \
        --build-arg BUILD_TYPE=${BUILD_TYPE}"

    if [ "${NO_CACHE}" = true ]; then
        DOCKER_BUILD_COMMAND+=" --no-cache"
    fi

    DOCKER_BUILD_COMMAND+=" -t ${IMAGE_TAG} ."

    if [[ -n "$LOG_FILE" ]]; then
        eval "$DOCKER_BUILD_COMMAND" >>"$LOG_FILE" 2>&1
    else
        eval "$DOCKER_BUILD_COMMAND"
    fi

    if [[ $? -ne 0 ]]; then
        echo "Error: Docker build failed. Exiting..."
        exit 1
    fi

    debug_log "Docker image built successfully. Pushing image ${IMAGE_TAG}"
    docker push ${IMAGE_TAG}
    if [[ $? -ne 0 ]]; then
        echo "Error: Docker push failed for ${IMAGE_TAG}. Exiting..."
        exit 1
    fi

    debug_log "Docker image ${IMAGE_TAG} pushed successfully."
}

main() {
    # Default settings
    DEBUG=false
    LOG_FILE=""
    BUILD_TYPE="release"
    NO_CACHE=false
    ARCH=""

    parse_arguments "$@"
    build_docker_image
}

main "$@"
