# Set the current script's directory as the project root
$projectRoot = $PSScriptRoot

# Add the current path to the environment variable
# Write-Output "Adding '$projectRoot' to the PATH environment variable for this session..."
# $env:PATH = "$projectRoot;$env:PATH"
# Write-Output "Current PATH: $env:PATH"

# Define backend directory
$backendDir = "$projectRoot/gallery-backend"

try {
    # Navigate to the backend directory
    Set-Location -Path $backendDir

    # Execute the backend
    Write-Output "Starting the backend using 'cargo run --release'..."
    cargo run --release
}
catch {
    # Handle failure gracefully
    Write-Output "Failed to run the backend. Check the error logs."
}
finally {
    # Always return to the root directory
    Set-Location -Path $projectRoot
    Write-Output "Returned to the root directory: $projectRoot"
}