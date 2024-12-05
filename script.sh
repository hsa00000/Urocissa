#!/bin/bash

# Get the absolute path of this script
SCRIPT_DIR=$(dirname "$(realpath "$0")")

# Set the UROCISSA_PATH to the script's absolute path
UROCISSA_PATH="$SCRIPT_DIR"

# Retrieve the branch name and last commit hash of optimize/use-cargo-chef
BRANCH="main"
LAST_COMMIT_HASH=$(git rev-parse "$BRANCH")

if [[ -z "$LAST_COMMIT_HASH" ]]; then
    echo "Error: Unable to retrieve the last commit hash for branch $BRANCH."
    exit 1
fi

# Print the branch and commit hash for verification
echo "Branch: $BRANCH"
echo "Last Commit Hash: $LAST_COMMIT_HASH"

# Update the Docker build command to include the new build arguments
DOCKER_BUILD_COMMAND="sudo docker build \
    --build-arg UROCISSA_PATH=${UROCISSA_PATH} \
    --build-arg LAST_COMMIT_HASH=${LAST_COMMIT_HASH} \
    --build-arg BRANCH=${BRANCH} \
    -t urocissa ."

eval "$DOCKER_BUILD_COMMAND"

# Set the path of the .env file
ENV_FILE="./gallery-backend/.env"
TEMP_ENV_FILE="./gallery-backend/temp.env"

# Initialize volumes array
PREDEFINED_VOLUMES=()
DYNAMIC_VOLUMES=()

# Function to ensure config files exist and add to volume mounts
ensure_config_file() {
    local source_file="$1"
    local target_file="$2"
    local volume_path="${3:-$target_file}"

    if [[ ! -f "$target_file" ]]; then
        echo "$target_file not found. Copying from ${source_file}."
        cp "$source_file" "$target_file"
    fi

    # Add to predefined volumes if a volume path is provided
    if [[ -n "$volume_path" ]]; then
        PREDEFINED_VOLUMES+=("$target_file:$volume_path")
    fi
}

# Ensure necessary config files exist and set up volume mounts
ensure_config_file "./gallery-backend/Rocket.default.toml" "./gallery-backend/Rocket.toml" "${UROCISSA_PATH}/gallery-backend/Rocket.toml"
ensure_config_file "./gallery-frontend/config.default.ts" "./gallery-frontend/config.ts" "${UROCISSA_PATH}/gallery-frontend/config.ts"
ensure_config_file "./gallery-backend/.env.default" "$ENV_FILE" "${UROCISSA_PATH}/gallery-backend/.env"

# Process SYNC_PATH for dynamic volume mounts
SYNC_PATH=$(grep -E '^SYNC_PATH\s*=\s*' "$ENV_FILE" | sed 's/^SYNC_PATH\s*=\s*//')

# Process SYNC_PATH if it's not empty
if [[ -n "$SYNC_PATH" ]]; then
    # If the value has quotes, remove them
    SYNC_PATH="${SYNC_PATH%\"}"
    SYNC_PATH="${SYNC_PATH#\"}"

    echo "Original SYNC_PATH is: $SYNC_PATH"

    # Split SYNC_PATH by commas and convert to an array
    IFS=',' read -ra PATHS <<< "$SYNC_PATH"

    ABS_PATHS=()

    # Get the directory where the .env file is located
    ENV_DIR=$(dirname "$ENV_FILE")

    for path in "${PATHS[@]}"; do
        # Remove leading and trailing spaces
        trimmed_path=$(echo "$path" | xargs)

        # Determine the absolute path
        if [[ "$trimmed_path" = /* ]]; then
            abs_path="$trimmed_path"
        else
            abs_path=$(realpath -m "$ENV_DIR/$trimmed_path")
        fi

        # Append to ABS_PATHS (if needed elsewhere)
        ABS_PATHS+=("$abs_path")

        # Prepare and append the dynamic volume mount
        DYNAMIC_VOLUMES+=("$abs_path:$abs_path")
    done
else
    echo "Warning: SYNC_PATH variable not found or is empty in $ENV_FILE. Skipping dynamic volume mounts."
fi

# Additional predefined volumes
PREDEFINED_VOLUMES+=(
    "./gallery-backend/db:${UROCISSA_PATH}/gallery-backend/db"
    "./gallery-backend/object:${UROCISSA_PATH}/gallery-backend/object"
)

# Build the Docker image with UROCISSA_PATH as a build argument
echo "Building Docker image with UROCISSA_PATH set to $UROCISSA_PATH"
sudo docker build --build-arg UROCISSA_PATH=$UROCISSA_PATH -t urocissa .

# Prepare formatted predefined volume mount output
PREDEFINED_VOLUME_OUTPUT=""
for vol in "${PREDEFINED_VOLUMES[@]}"; do
    PREDEFINED_VOLUME_OUTPUT+=" \\
    -v \"$vol\""
done

# Prepare formatted dynamic volume mount output
DYNAMIC_VOLUME_OUTPUT=""
for vol in "${DYNAMIC_VOLUMES[@]}"; do
    DYNAMIC_VOLUME_OUTPUT+=" \\
    -v \"$vol\""
done

# Read port from Rocket.toml
ROCKET_PORT=$(grep -E '^port\s*=\s*' ./gallery-backend/Rocket.toml | sed 's/^port\s*=\s*//')

# If port not found, use default port 4000
if [[ -z "$ROCKET_PORT" ]]; then
    ROCKET_PORT=4000
    echo "Port not found in Rocket.toml. Using default port 4000."
fi

# Final Docker Run command
DOCKER_RUN_COMMAND="sudo docker run -it --rm \\
${PREDEFINED_VOLUME_OUTPUT} \\
${DYNAMIC_VOLUME_OUTPUT} \\
    -p ${ROCKET_PORT}:${ROCKET_PORT} urocissa"

# Output the final Docker Run command
echo -e "\nGenerated Docker Run command:\n"
echo "$DOCKER_RUN_COMMAND"

# Execute the Docker Run command immediately
echo -e "\nExecuting Docker Run command...\n"
eval "$DOCKER_RUN_COMMAND"

# Check if the Docker Run command was successful
if [[ $? -ne 0 ]]; then
    echo "Error: Docker Run command failed to execute"
    exit 1
else
    echo "Docker container has been successfully started"
fi