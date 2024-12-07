#!/bin/bash

# Files to remove
files=(
    "./gallery-backend/.env"
    "./gallery-backend/Rocket.toml"
    "./gallery-frontend/config.ts"
    "./build.log"
)

# Folders to remove
folders=(
    "./gallery-backend/db"
    "./gallery-backend/object"
    "./gallery-backend/target"
    "./gallery-backend/upload"
)

# Remove specified files
echo "Removing specified files..."
for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        rm "$file"
        echo "Removed: $file"
    else
        echo "File not found: $file"
    fi
done

# Remove specified folders and their contents
echo "Removing specified folders and their contents..."
for folder in "${folders[@]}"; do
    if [ -d "$folder" ]; then
        rm -rf "$folder"
        echo "Removed folder: $folder"
    else
        echo "Folder not found: $folder"
    fi
done

# Remove all Docker containers and images
echo "Removing Docker containers and images..."
docker rm -f $(docker ps -aq) 2>/dev/null || echo "No containers to remove."
docker rmi -f $(docker images -q) 2>/dev/null || echo "No images to remove."

echo "Cleanup completed."
