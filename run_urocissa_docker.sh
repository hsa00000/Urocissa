#!/bin/bash

# ============================================================
# Color & Logging Configuration
# ============================================================
# Define ANSI color codes for terminal output formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color (Reset)

# Logs a prominent header for script sections
log_header() {
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}# $1${NC}"
    echo -e "${BLUE}============================================================${NC}"
}

log_info() { echo -e "${CYAN}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Logs debug messages only if DEBUG mode is enabled
log_debug() {
    if [[ "$DEBUG" == true ]]; then
        local msg="[DEBUG] $1"
        if [[ -n "$LOG_FILE" ]]; then
            echo "$msg" >>"$LOG_FILE"
        fi
        echo -e "${YELLOW}$msg${NC}"
    fi
}

# ============================================================
# Function Definitions
# ============================================================

# Displays script usage instructions
show_help() {
    cat <<EOF
Usage: ./run_urocissa_docker.sh [OPTIONS]

Description:
  This script runs the Urocissa Docker container with automated configuration 
  migration and enhanced logging.

Options:
  --help             Show this help message and exit.
  --debug            Enable debug mode (verbose output).
  --dev              Use the 'dev' tag (hsa00000/urocissa:dev).
  --tag <tag>        Use a specific image tag (for example, 3.3.1).
  --log-file <file>  Specify a log file for debug output.

Examples:
  ./run_urocissa_docker.sh
  ./run_urocissa_docker.sh --tag 3.3.1
  ./run_urocissa_docker.sh --debug --log-file run.log
EOF
}

# Checks for the existence of a config file and adds it to the Docker volume list
ensure_config_file() {
    local target_file="$1"
    local volume_path="${2:-$target_file}"
    
    if [[ -f "$target_file" ]]; then
        if [[ -n "$volume_path" ]]; then
            PREDEFINED_VOLUMES+=("$target_file:$volume_path")
            log_success "Config found: $target_file -> Mounted"
        fi
    else
        log_warn "Config missing: $target_file -> Skipped"
    fi
}

# Parses command line arguments provided to the script
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
            --dev)
                DOCKER_TAG="dev"
                shift
                ;;
            --tag)
                if [[ $# -lt 2 || -z "$2" || "$2" == --* ]]; then
                    log_error "Option --tag requires an image tag"
                fi
                DOCKER_TAG="$2"
                shift 2
                ;;
            --log-file)
                LOG_FILE="$2"
                if ! : > "$LOG_FILE"; then
                    log_error "Failed to initialize log file at $LOG_FILE"
                fi
                shift 2
                ;;
            *)
                log_error "Unknown option $1"
                ;;
        esac
    done
}

# Step 1: Pre-run environment setup and basic directory checks
setup_environment() {
    log_header "Step 1: Environment Setup"
    
    SCRIPT_DIR=$(dirname "$(realpath "$0")")
    UROCISSA_PATH="$SCRIPT_DIR"
    log_info "Project Root: $UROCISSA_PATH"

    ENV_FILE="./gallery-backend/.env"

    # Initialize volume arrays
    PREDEFINED_VOLUMES=()
    DYNAMIC_VOLUMES=()

    log_info "Checking configuration files..."

    # Create an empty config.json if it doesn't exist to prevent mounting errors
    if [[ ! -f "./gallery-backend/config.json" ]]; then
        echo "{}" > "./gallery-backend/config.json"
        log_success "Created empty config.json"
    fi
    ensure_config_file "./gallery-backend/config.json" "${UROCISSA_PATH}/gallery-backend/config.json"

    # Mount legacy config files to allow the application to migrate settings internally
    ensure_config_file "./gallery-backend/.env" "${UROCISSA_PATH}/gallery-backend/.env"
    ensure_config_file "./gallery-backend/Rocket.toml" "${UROCISSA_PATH}/gallery-backend/Rocket.toml"

    # Ensure the required upload directory exists locally
    UPLOAD_DIR="./gallery-backend/upload"
    if [ ! -d "$UPLOAD_DIR" ]; then
        mkdir -p "$UPLOAD_DIR"
        log_success "Created directory: $UPLOAD_DIR"
    else
        log_success "Directory exists: $UPLOAD_DIR"
    fi

    # Convert Windows CRLF to Linux LF for the .env file to avoid parsing issues
    if [[ -f "$ENV_FILE" ]]; then
        sed -i 's/\r$//' "$ENV_FILE"
        log_debug "Converted CRLF to LF for $ENV_FILE"
    fi
}

# Step 2: Identify and prepare directory paths to be mounted as volumes
prepare_volumes() {
    log_header "Step 2: Volume & Path Parsing"

    local RAW_PATHS_LIST=()
    local CONFIG_SOURCE="None" 
    local CONFIG_FILE="./gallery-backend/config.json"
    
    # ------------------------------------------------------------
    # 1. Attempt to read 'syncPaths' from config.json (Modern Config)
    # ------------------------------------------------------------
    if [[ -f "$CONFIG_FILE" ]]; then
        local FLAT_JSON
        FLAT_JSON=$(tr -d '\n\r' < "$CONFIG_FILE")

        if echo "$FLAT_JSON" | grep -q '"syncPaths"'; then
             local JSON_CONTENT
             JSON_CONTENT=$(echo "$FLAT_JSON" | sed -e 's/.*"syncPaths"[[:space:]]*:[[:space:]]*\[//' -e 's/\].*//')
             
             if [[ -n "$JSON_CONTENT" ]]; then
                CONFIG_SOURCE="config.json"
                IFS=',' read -ra JSON_ARR <<< "$JSON_CONTENT"
                for item in "${JSON_ARR[@]}"; do
                    # Clean up quotes, whitespace, and normalize Windows slashes (\\ to /)
                    local cleaned_path
                    cleaned_path=$(echo "$item" | tr -d '"' | xargs | sed 's|\\\\|/|g' | sed 's|\\|/|g')
                    
                    if [[ -n "$cleaned_path" ]]; then
                        RAW_PATHS_LIST+=("$cleaned_path")
                    fi
                done
             fi
        fi
    fi

    # ------------------------------------------------------------
    # 2. Fallback: Read SYNC_PATH from legacy .env
    # ------------------------------------------------------------
    if [[ "$CONFIG_SOURCE" == "None" ]]; then
        if [[ -f "./gallery-backend/.env" ]]; then
            SYNC_PATH=$(grep -E '^SYNC_PATH\s*=\s*' ./gallery-backend/.env | sed 's/^SYNC_PATH\s*=\s*//')
            if [[ -n "$SYNC_PATH" ]]; then
                CONFIG_SOURCE=".env"
                SYNC_PATH="${SYNC_PATH%\"}" # Remove trailing quote
                SYNC_PATH="${SYNC_PATH#\"}" # Remove leading quote

                IFS=',' read -ra PATHS <<<"$SYNC_PATH"
                for path in "${PATHS[@]}"; do
                    local cleaned_path
                    cleaned_path=$(echo "$path" | xargs)
                    if [[ -n "$cleaned_path" ]]; then
                        RAW_PATHS_LIST+=("$cleaned_path")
                    fi
                done
            fi
        fi
    fi

    # ------------------------------------------------------------
    # 3. Log detection results
    # ------------------------------------------------------------
    echo "------------------------------------------------------------"
    echo -e " ${CYAN}Sync Path Source :${NC} $CONFIG_SOURCE"
    if [ ${#RAW_PATHS_LIST[@]} -eq 0 ]; then
         echo -e " ${YELLOW}Paths Found      :${NC} (None)"
    else
         echo -e " ${GREEN}Paths Found      :${NC}"
         for raw_p in "${RAW_PATHS_LIST[@]}"; do
             echo "   - $raw_p"
         done
    fi
    echo "------------------------------------------------------------"

    # ------------------------------------------------------------
    # 4. Convert discovered paths to absolute paths for Docker
    # ------------------------------------------------------------
    for path in "${RAW_PATHS_LIST[@]}"; do
        local abs_path
        
        if [[ "$path" = /* ]]; then
            abs_path=$(realpath -m "$path")
        else
            abs_path=$(realpath -m "./gallery-backend/$path")
        fi

        log_debug "Resolving: $path -> $abs_path"

        # Verification: If the path doesn't exist locally, Docker will mount it as a directory
        if [[ ! -e "$abs_path" ]]; then
            echo ""
            log_error "Path not found!\n    Source: $CONFIG_SOURCE\n    Raw:    $path\n    Abs:    $abs_path"
        fi

        DYNAMIC_VOLUMES+=("$abs_path:$abs_path")
    done

    # ------------------------------------------------------------
    # 5. Define persistent database and object storage volumes
    # ------------------------------------------------------------
    PREDEFINED_VOLUMES+=( "./gallery-backend/db:${UROCISSA_PATH}/gallery-backend/db" )
    PREDEFINED_VOLUMES+=( "./gallery-backend/object:${UROCISSA_PATH}/gallery-backend/object" )
    
    log_success "Volume preparation complete."
}

# Step 3: Identify the port and execute the Docker container
run_container() {
    log_header "Step 3: Docker Execution"

    # Default port
    local HOST_PORT=5673

    # Priority 1: Read internal port from config.json
    if [[ -f "./gallery-backend/config.json" ]]; then
        local detected_port
        detected_port=$(grep -o '"port"\s*:\s*[0-9]*' ./gallery-backend/config.json | head -n 1 | grep -o '[0-9]*')
        if [[ -n "$detected_port" ]]; then
            HOST_PORT="$detected_port"
        fi
    fi

    # Priority 2: Check Rocket.toml for user-defined external port
    if [[ -f "./gallery-backend/Rocket.toml" ]]; then
        local rocket_port
        rocket_port=$(grep -E '^port\s*=\s*' ./gallery-backend/Rocket.toml | sed -E 's/^port\s*=\s*"?([0-9]+)"?/\1/' | tr -d '[:space:]')
        if [[ -n "$rocket_port" ]]; then
            HOST_PORT="$rocket_port"
        fi
    fi

    log_info "Service Port : $HOST_PORT"
    log_info "Image Tag    : $DOCKER_TAG"

    # Construct the Docker command using an array for safe handling of special characters
    local cmd=(docker run -it --rm)
    
    for vol in "${PREDEFINED_VOLUMES[@]}"; do
        cmd+=(-v "$vol")
    done
    for vol in "${DYNAMIC_VOLUMES[@]}"; do
        cmd+=(-v "$vol")
    done

    cmd+=(-e "UROCISSA_PATH=${UROCISSA_PATH}")
    cmd+=(-p "${HOST_PORT}:${HOST_PORT}")
    cmd+=( "hsa00000/urocissa:${DOCKER_TAG}" )

    # Output the final command for debugging/manual copying
    echo ""
    echo -e "${YELLOW}┌──────────────────────┐${NC}"
    echo -e "${YELLOW}│ Final Docker Command │${NC}"
    echo -e "${YELLOW}└──────────────────────┘${NC}"
    printf "%q " "${cmd[@]}"
    echo -e "\n"

    log_info "Starting container..."
    "${cmd[@]}"
    local exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        log_error "Docker container exited with error code $exit_code"
    fi
    
    return $exit_code
}

# Step 4: Cleanup legacy configuration files after successful migration
cleanup_legacy_files() {
    log_header "Step 4: Legacy File Cleanup"
    
    local cleaned=false
    
    if [[ -f "./gallery-backend/.env" ]]; then
        rm -f "./gallery-backend/.env"
        log_success "Removed legacy .env file"
        cleaned=true
    fi
    
    if [[ -f "./gallery-backend/Rocket.toml" ]]; then
        rm -f "./gallery-backend/Rocket.toml"
        log_success "Removed legacy Rocket.toml file"
        cleaned=true
    fi
    
    if [[ "$cleaned" == false ]]; then
        log_info "No legacy files to clean up"
    fi
}

# Main script entry point
main() {
    DEBUG=false
    LOG_FILE=""
    DOCKER_TAG="latest"

    # Trap Ctrl+C (SIGINT) to handle user interruption gracefully
    trap 'echo -e "\n${RED}[ABORT] Script interrupted by user.${NC}"; exit 1' INT

    parse_arguments "$@"
    
    setup_environment
    prepare_volumes
    run_container
    local container_exit=$?
    
    # Cleanup only triggers if the container was launched
    cleanup_legacy_files
    
    if [[ $container_exit -eq 0 ]]; then
        log_header "Execution Finished Successfully"
    fi
}

main "$@"
