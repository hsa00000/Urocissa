#!/bin/bash

# ============================================================
# Function Definitions
# ============================================================

show_help() {
    cat <<EOF
Usage: ./run_urocissa_docker.sh [OPTIONS]

Description:
  This script runs the Urocissa Docker container from an already built image.
  It sets up environment variables, volumes, and port mappings based on the local configuration files.

Options:
  --help              Show this help message and exit.
  --debug             Enable debug mode to display additional information during execution.
  --log-file <file>   Specify a log file for debug output. The file will be created if it does not exist,
                      or cleared if it already exists.

Examples:
  1. Run the container with default settings:
     ./run_urocissa_docker.sh

  2. Enable debug mode and specify a log file:
     ./run_urocissa_docker.sh --debug --log-file run.log

Notes:
  - Ensure that the Docker image 'urocissa' is already built by running ./build_urocissa_docker.sh beforehand.
  - Debug mode outputs information to the terminal unless a log file is specified.
  - The script will mount local directories and set UROCISSA_PATH based on the current directory structure.
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

ensure_config_file() {
    # This function ensures that certain config files exist before running.
    # It's retained for logging purposes but here we assume the files already exist as they should be created before build.
    local source_file="$1"
    local target_file="$2"
    local volume_path="${3:-$target_file}"
    if [[ ! -f "$target_file" ]]; then
        debug_log "$target_file not found. Attempting to restore from $source_file."
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
        --log-file)
            LOG_FILE="$2"
            >"$LOG_FILE"
            if [[ $? -ne 0 ]]; then
                echo "Error: Failed to initialize log file at $LOG_FILE"
                exit 1
            fi
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

    ENV_FILE="./gallery-backend/.env"

    # Initialize arrays
    PREDEFINED_VOLUMES=()
    DYNAMIC_VOLUMES=()

    # Ensure config files exist and mount them
    ensure_config_file "./gallery-backend/Rocket.default.toml" "./gallery-backend/Rocket.toml" "${UROCISSA_PATH}/gallery-backend/Rocket.toml"
    ensure_config_file "./gallery-frontend/config.default.ts" "./gallery-frontend/config.ts" "${UROCISSA_PATH}/gallery-frontend/config.ts"
    ensure_config_file "./gallery-backend/config.default.json" "./gallery-backend/config.json" "${UROCISSA_PATH}/gallery-backend/config.json"
    ensure_config_file "./gallery-backend/.env.default" "$ENV_FILE" "${UROCISSA_PATH}/gallery-backend/.env"

    # Convert CRLF to LF
    sed -i 's/\r$//' "$ENV_FILE"
}

prepare_volumes() {
    # Process SYNC_PATH for dynamic volume mounts
    SYNC_PATH=$(grep -E '^SYNC_PATH\s*=\s*' ./gallery-backend/.env | sed 's/^SYNC_PATH\s*=\s*//')
    if [[ -n "$SYNC_PATH" ]]; then
        SYNC_PATH="${SYNC_PATH%\"}"
        SYNC_PATH="${SYNC_PATH#\"}"
        debug_log "Original SYNC_PATH is: $SYNC_PATH"

        IFS=',' read -ra PATHS <<<"$SYNC_PATH"
        for path in "${PATHS[@]}"; do
            trimmed_path=$(echo "$path" | xargs)
            if [[ "$trimmed_path" = /* ]]; then
                abs_path=$(realpath -m "$trimmed_path")
            else
                abs_path=$(realpath -m "$(dirname "./gallery-backend/.env")/$trimmed_path")
            fi
            DYNAMIC_VOLUMES+=("$abs_path:$abs_path")
        done
    else
        debug_log "Warning: SYNC_PATH variable not found or is empty in .env. Skipping dynamic volume mounts."
    fi

    PREDEFINED_VOLUMES+=( "./gallery-backend/db:${UROCISSA_PATH}/gallery-backend/db" )
    PREDEFINED_VOLUMES+=( "./gallery-backend/object:${UROCISSA_PATH}/gallery-backend/object" )

    debug_log "Predefined volumes: ${PREDEFINED_VOLUMES[*]}"
    debug_log "Dynamic volumes: ${DYNAMIC_VOLUMES[*]}"
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

    DOCKER_RUN_COMMAND+=" -e UROCISSA_PATH=${UROCISSA_PATH}"
    DOCKER_RUN_COMMAND+=" -p ${ROCKET_PORT}:${ROCKET_PORT} hsa00000/urocissa:latest"

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
    DEBUG=false
    LOG_FILE=""

    parse_arguments "$@"
    setup_environment
    prepare_volumes
    run_container
}

main "$@"
