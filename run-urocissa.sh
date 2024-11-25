#!/bin/bash

# Set the current script's directory as the project root
PROJECT_ROOT=$(dirname "$(readlink -f "$0")")
BACKEND_DIR="$PROJECT_ROOT/gallery-backend"

# Function to gracefully handle errors
handle_error() {
    echo "Failed to run the backend. Check the error logs."
    cd "$PROJECT_ROOT" || exit
    echo "Returned to the root directory: $PROJECT_ROOT"
    exit 1
}

# Navigate to the backend directory
echo "Navigating to the backend directory: $BACKEND_DIR"
cd "$BACKEND_DIR" || handle_error

# Execute the backend
echo "Starting the backend using 'cargo run --release'..."
if cargo run --release; then
    echo "Backend is running successfully."
else
    handle_error
fi

# Always return to the root directory
cd "$PROJECT_ROOT" || exit
echo "Returned to the root directory: $PROJECT_ROOT"
