#!/bin/bash

# ============================================================
# Function Definitions
# ============================================================

show_help() {
    cat <<EOF
Usage: ./run_urocissa_docker.sh [OPTIONS]

Description:
  This script simplifies the process of running a Docker image for the Urocissa project.
  It supports options for debugging, logging, specifying build types (release, debug, or custom profiles),
  and controlling Docker caching behavior.

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

Examples:
  1. Run with default settings (release build):
     ./run_urocissa_docker.sh

  2. Enable debug mode and specify a log file:
     ./run_urocissa_docker.sh --debug --log-file build.log

  3. Build with debug configuration:
     ./run_urocissa_docker.sh --build-type debug

  4. Build with a custom profile (e.g., dev-release):
     ./run_urocissa_docker.sh --build-type dev-release

  5. Disable Docker cache during build:
     ./run_urocissa_docker.sh --no-cache

  6. Combine debug mode, log file, custom build type, and disable cache:
     ./run_urocissa_docker.sh --debug --log-file debug.log --build-type dev-release --no-cache

Notes:
  - The log file specified with --log-file will be initialized at the start.
  - Debug mode outputs information to the terminal unless a log file is specified.
  - If --build-type is not specified, the default is "release".
  - The --build-type option supports custom profiles as defined in Cargo.toml.
  - The --no-cache option ensures no intermediate layers are used from previous builds.

Exit Codes:
  0  Success
  1  Error occurred during execution
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

ensure_config_file() {
    local source_file="$1"
    local target_file="$2"
    local volume_path="${3:-$target_file}"
    if [[ ! -f "$target_file" ]]; then
        debug_log "$target_file not found. Copying from $source_file."

        # Due to an unknown issue in WSL, `cp "$source_file" "$target_file"`
        # sometimes does not work as expected. To ensure reliability, we use `mv`
        # to rename the source file first, followed by `cp` to create a new copy.
        mv "$source_file" "$target_file"
        cp "$target_file" "$source_file"
    fi
    if [[ -n "$volume_path" ]]; then
        PREDEFINED_VOLUMES+=("$target_file:$volume_path")
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
        *)
            echo "Error: Unknown option $1"
            exit 1
            ;;
        esac
    done
}

setup_environment() {
    SCRIPT_DIR=$(dirname "$(realpath "$0")")
    UROCISSA_PATH="$SCRIPT_DIR"

    debug_log "Script directory set to $SCRIPT_DIR"
    debug_log "Build type is set to $BUILD_TYPE"

    ENV_FILE="./gallery-backend/.env"

    # Initialize arrays
    PREDEFINED_VOLUMES=()
    DYNAMIC_VOLUMES=()

    # Ensure config files
    ensure_config_file "./gallery-backend/Rocket.default.toml" "./gallery-backend/Rocket.toml" "${UROCISSA_PATH}/gallery-backend/Rocket.toml"
    ensure_config_file "./gallery-frontend/config.default.ts" "./gallery-frontend/config.ts" "${UROCISSA_PATH}/gallery-frontend/config.ts"
    ensure_config_file "./gallery-backend/config.default.json" "./gallery-backend/config.json" "${UROCISSA_PATH}/gallery-backend/config.json"
    ensure_config_file "./gallery-backend/.env.default" "$ENV_FILE" "${UROCISSA_PATH}/gallery-backend/.env"

    # Convert CRLF to LF
    sed -i 's/\r$//' "$ENV_FILE"
}

prepare_volumes() {
    # Process SYNC_PATH for dynamic volume mounts
    SYNC_PATH=$(grep -E '^SYNC_PATH\s*=\s*' "$ENV_FILE" | sed 's/^SYNC_PATH\s*=\s*//')
    if [[ -n "$SYNC_PATH" ]]; then
        SYNC_PATH="${SYNC_PATH%\"}"
        SYNC_PATH="${SYNC_PATH#\"}"
        debug_log "Original SYNC_PATH is: $SYNC_PATH"

        IFS=',' read -ra PATHS <<<"$SYNC_PATH"
        for path in "${PATHS[@]}"; do
            trimmed_path=$(echo "$path" | xargs)
            # Check if the path is absolute
            if [[ "$trimmed_path" = /* ]]; then
                abs_path=$(realpath -m "$trimmed_path")
            else
                abs_path=$(realpath -m "$(dirname "$ENV_FILE")/$trimmed_path")
            fi
            DYNAMIC_VOLUMES+=("$abs_path:$abs_path")
        done
    else
        debug_log "Warning: SYNC_PATH variable not found or is empty in $ENV_FILE. Skipping dynamic volume mounts."
    fi

    PREDEFINED_VOLUMES+=(
        "./gallery-backend/db:${UROCISSA_PATH}/gallery-backend/db"
        "./gallery-backend/object:${UROCISSA_PATH}/gallery-backend/object"
    )

    debug_log "Predefined volumes: ${PREDEFINED_VOLUMES[*]}"
    debug_log "Dynamic volumes: ${DYNAMIC_VOLUMES[*]}"
}

build_docker_image() {
    debug_log "Building Docker image with BUILD_TYPE=$BUILD_TYPE"

    DOCKER_BUILD_COMMAND="docker build \
        --build-arg BUILD_TYPE=${BUILD_TYPE}"

    if [ "${NO_CACHE}" = true ]; then
        DOCKER_BUILD_COMMAND+=" --no-cache"
    fi

    DOCKER_BUILD_COMMAND+=" -t urocissa ."

    if [[ -n "$LOG_FILE" ]]; then
        eval "$DOCKER_BUILD_COMMAND" >>"$LOG_FILE" 2>&1
    else
        eval "$DOCKER_BUILD_COMMAND"
    fi

    if [[ $? -ne 0 ]]; then
        echo "Error: Docker build failed. Exiting..."
        exit 1
    fi
}

run_container() {
    # Extract port from Rocket.toml
    ROCKET_PORT=$(grep -E '^port\s*=\s*' ./gallery-backend/Rocket.toml | sed -E 's/^port\s*=\s*"?([0-9]+)"?/\1/' | tr -d '[:space:]')
    ROCKET_PORT=${ROCKET_PORT:-4000}
    debug_log "Using port: $ROCKET_PORT"

    # Generate Docker Run command
    DOCKER_RUN_COMMAND="docker run -it --rm"
    for vol in "${PREDEFINED_VOLUMES[@]}"; do
        DOCKER_RUN_COMMAND+=" -v $vol"
    done
    for vol in "${DYNAMIC_VOLUMES[@]}"; do
        DOCKER_RUN_COMMAND+=" -v $vol"
    done

    # Pass UROCISSA_PATH at runtime as an environment variable
    DOCKER_RUN_COMMAND+=" -e UROCISSA_PATH=${UROCISSA_PATH}"
    DOCKER_RUN_COMMAND+=" -p ${ROCKET_PORT}:${ROCKET_PORT} urocissa"

    debug_log "Generated Docker Run command: $DOCKER_RUN_COMMAND"
    eval "$DOCKER_RUN_COMMAND"

    if [[ $? -ne 0 ]]; then
        echo "Error: Docker Run command failed to execute"
        exit 1
    else
        debug_log "Docker container has been successfully started"
    fi
}

main() {
    # Default settings
    DEBUG=false
    LOG_FILE=""
    BUILD_TYPE="release"
    NO_CACHE=false

    parse_arguments "$@"
    setup_environment
    prepare_volumes
    build_docker_image
    run_container
}

# ============================================================
# Execute main with all passed arguments
# ============================================================
main "$@"
