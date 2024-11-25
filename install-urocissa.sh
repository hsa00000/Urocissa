#!/bin/bash

# Enable debug mode if specified
DEBUG=false
UPDATE=false

while [[ "$#" -gt 0 ]]; do
    case $1 in
        --debug) DEBUG=true ;;
        --update) UPDATE=true ;;
    esac
    shift
done

# Define project directories
PROJECT_ROOT=$(dirname "$(readlink -f "$0")")
BACKEND_DIR="$PROJECT_ROOT/gallery-backend"
FRONTEND_DIR="$PROJECT_ROOT/gallery-frontend"

# Helper function to check if a command exists
command_exists() {
    command -v "$1" &>/dev/null
}

# Update mode
if [ "$UPDATE" = true ]; then
    echo "Update mode enabled. Performing update tasks..."

    # Step 1: Pull the latest changes
    echo "Pulling the latest changes from the repository..."
    cd "$PROJECT_ROOT" || exit 1
    if git pull; then
        echo "Repository updated successfully."
    else
        echo "Failed to pull the latest changes. Ensure Git is installed and you have access to the repository."
        exit 1
    fi

    # Step 2: Rebuild the frontend
    echo "Rebuilding the frontend..."
    cd "$FRONTEND_DIR" || exit 1
    if npm run build; then
        echo "Frontend rebuilt successfully."
    else
        echo "Failed to rebuild the frontend. Check the error logs."
        exit 1
    fi

    # Step 3: Rebuild the backend
    echo "Rebuilding the backend..."
    cd "$BACKEND_DIR" || exit 1
    if [ "$DEBUG" = true ]; then
        echo "Debug mode enabled. Running 'cargo build'..."
        cargo build
    else
        echo "Running 'cargo build --release'..."
        cargo build --release
    fi
    echo "Backend rebuilt successfully."

    cd "$PROJECT_ROOT" || exit 1
    echo "Update completed successfully!"
    exit 0
fi

# Install FFmpeg
if command_exists ffmpeg; then
    echo "FFmpeg is already installed."
    ffmpeg -version
else
    echo "FFmpeg is not installed. Installing FFmpeg..."
    sudo apt update && sudo apt install -y ffmpeg
    if command_exists ffmpeg; then
        echo "FFmpeg installed successfully."
    else
        echo "FFmpeg installation failed. Please check your system configuration."
        exit 1
    fi
fi

# Install Rust
if ! command_exists rustc; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    if command_exists cargo; then
        echo "Rust installed successfully."
    else
        echo "Rust installation failed. Please check your system configuration."
        exit 1
    fi
else
    # Get the locally installed Rust version
    localRustVersion=$(rustc --version | awk '{print $2}')
    echo "Rust is already installed (version $localRustVersion)."

    # Check for the latest Rust version using rustup
    echo "Checking for the latest Rust version..."
    rustupCheckOutput=$(rustup check)

    # Function to parse the latest stable version from rustup check output
    get_latest_stable_version() {
        local output="$1"
        if echo "$output" | grep -q "->"; then
            # Extract the latest version when an update is available
            echo "$output" | grep "^stable" | awk -F "->" '{print $2}' | xargs
        else
            # Extract the current version when up-to-date
            echo "$output" | grep "^stable" | awk -F ":" '{print $2}' | xargs
        fi
    }

    # Parse the latest stable version
    latestRustVersion=$(get_latest_stable_version "$rustupCheckOutput")

    if [ -n "$latestRustVersion" ]; then
        echo "Latest Rust version: $latestRustVersion"
    else
        echo "Unable to determine the latest Rust version. Please check manually."
        exit 1
    fi

    # Compare installed and latest versions
    if [ "$localRustVersion" == "$latestRustVersion" ]; then
        echo "Rust is up-to-date."
    else
        echo "Your Rust version ($localRustVersion) is outdated. Latest version is $latestRustVersion."
        read -p "Do you want to update Rust? (y/n): " updateChoice
        if [ "$updateChoice" == "y" ]; then
            rustup update
            echo "Rust has been updated to the latest version."
        else
            echo "Rust update skipped."
        fi
    fi
fi

# Check if Node.js is installed
if ! command_exists node; then
    echo "Node.js is not installed."

    # Install fnm only if it's not already installed
    if ! command_exists fnm; then
        echo "Installing fnm..."
        cargo install fnm
        if command_exists fnm; then
            echo "fnm installed successfully."
        else
            echo "fnm installation failed. Please check your system configuration."
            exit 1
        fi
    fi

    # Configure fnm environment
    echo "Configuring fnm environment..."
    eval "$(fnm env --use-on-cd)"

    # Install Node.js using fnm
    NODE_VERSION=22
    echo "Installing Node.js version $NODE_VERSION using fnm..."
    fnm use --install-if-missing "$NODE_VERSION"
    if command_exists node; then
        echo "Node.js installed successfully."
        node --version
    else
        echo "Node.js installation failed. Please check your fnm configuration."
        exit 1
    fi
else
    echo "Node.js is already installed."
    node --version
fi

# Configure backend settings
echo "Configuring backend settings..."
cd "$BACKEND_DIR" || exit 1

if [ ! -f .env ]; then
    cp .env.default .env
    echo "Copied .env.default to .env."
else
    echo ".env file already exists. Skipping."
fi

if [ ! -f Rocket.toml ]; then
    cp Rocket.default.toml Rocket.toml
    echo "Copied Rocket.default.toml to Rocket.toml."
else
    echo "Rocket.toml file already exists. Skipping."
fi

# Build the backend
echo "Building the backend..."
if [ "$DEBUG" = true ]; then
    echo "Debug mode enabled. Running 'cargo build'..."
    cargo build
else
    echo "Running 'cargo build --release'..."
    cargo build --release
fi

# Configure frontend settings
echo "Configuring frontend settings..."
cd "$FRONTEND_DIR" || exit 1

if [ ! -f config.ts ]; then
    cp config.default.ts config.ts
    echo "Copied config.default.ts to config.ts."
else
    echo "config.ts file already exists. Skipping."
fi

# Build the frontend
echo "Building the frontend..."
if npm run build; then
    echo "Frontend built successfully."
else
    echo "Failed to build the frontend. Check the error logs."
    exit 1
fi

cd "$PROJECT_ROOT" || exit 1
echo "Setup complete. Your project is ready for use!"
