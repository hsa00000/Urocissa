param (
    [switch]$Debug,
    [switch]$Update
)

# Define project directories
$projectRoot = $PSScriptRoot
$backendDir = "$projectRoot/gallery-backend"
$frontendDir = "$projectRoot/gallery-frontend"


# If update mode is enabled, perform update tasks and exit
if ($Update) {
    Write-Output "Update mode enabled. Performing update tasks..."

    # Step 1: Pull the latest changes
    Write-Output "Pulling the latest changes from the repository..."
    try {
        Set-Location -Path $projectRoot
        git pull
        Write-Output "Repository updated successfully."
    }
    catch {
        Write-Output "Failed to pull the latest changes. Ensure Git is installed and you have access to the repository."
        exit 1
    }

    # Step 3: Rebuild the backend
    Write-Output "Rebuilding the backend..."
    try {
        Set-Location -Path $backendDir
        if ($Debug) {
            Write-Output "Debug mode enabled. Running 'cargo build'..."
            cargo build
        }
        else {
            Write-Output "Running 'cargo build --release'..."
            cargo build --release
        }
        Write-Output "Backend rebuilt successfully."
    }
    catch {
        Write-Output "Failed to rebuild the backend. Check the error logs."
        exit 1
    }

    # Step 2: Rebuild the frontend
    Write-Output "Rebuilding the frontend..."
    try {
        Set-Location -Path $frontendDir
        npm run build
        Write-Output "Frontend rebuilt successfully."
    }
    catch {
        Write-Output "Failed to rebuild the frontend. Check the error logs."
        exit 1
    }

    
    # Return to the root directory
    Set-Location -Path $projectRoot
    Write-Output "Update completed successfully!"
    return
}




# Helper function to check if a command exists
function CommandExists {
    param ($command)
    # Check for the local command first
    if (Test-Path ".\$command.exe") {
        return $true
    }
    else {
        return Get-Command $command -ErrorAction SilentlyContinue
    }
}

# Install FFmpeg
if (CommandExists "ffmpeg") {
    # Try to display the FFmpeg version
    if (Test-Path ".\ffmpeg.exe") {
        $output = .\ffmpeg.exe -version
    }
    else {
        $output = ffmpeg -version
    }

    # Extract the core version number
    $coreVersion = $output | Select-String -Pattern "ffmpeg version (\d+\.\d+)" | ForEach-Object {
    ($_ -match "ffmpeg version (\d+\.\d+)") | Out-Null
        $matches[1]
    }

    Write-Output "FFmpeg is already installed (version $coreVersion)."
}
else {
    Write-Output "FFmpeg is not installed and ffmpeg.exe is not found in the root directory. Installing FFmpeg..."

    $ffmpegUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
    $ffmpegArchiveName = "ffmpeg.zip"

    Write-Output "Downloading FFmpeg from $ffmpegUrl..."
    Invoke-WebRequest -Uri $ffmpegUrl -OutFile $ffmpegArchiveName -ErrorAction Stop

    Write-Output "Extracting FFmpeg..."
    Expand-Archive -Path $ffmpegArchiveName -DestinationPath "." -Force

    Write-Output "Organizing FFmpeg files..."
    Get-ChildItem -Path ".\ffmpeg-*-essentials_build\bin\ffmpeg.exe" -Recurse | ForEach-Object {
        Move-Item -Path $_.FullName -Destination ".\ffmpeg.exe"
    }

    Write-Output "Cleaning up temporary files..."
    Remove-Item -Recurse -Force ".\ffmpeg-*-essentials_build"
    Remove-Item -Force $ffmpegArchiveName

    Write-Output "FFmpeg setup complete! The binary is located in the root directory."
}

# Install Rust
if (-not (CommandExists "cargo")) {
    Write-Output "Rust is not installed. Installing Rust..."

    # URLs for Rust installers
    $rustInstaller64 = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
    $rustInstaller32 = "https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe"

    # Determine system architecture
    if ([System.Environment]::Is64BitOperatingSystem) {
        Write-Output "64-bit operating system detected."
        $rustInstallerUrl = $rustInstaller64
    }
    else {
        Write-Output "32-bit operating system detected."
        $rustInstallerUrl = $rustInstaller32
    }

    # Download the appropriate installer
    $installerPath = "rustup-init.exe"
    Write-Output "Downloading Rust installer from $rustInstallerUrl..."
    Invoke-WebRequest -Uri $rustInstallerUrl -OutFile $installerPath -ErrorAction Stop

    Write-Output "Rust installer downloaded successfully to $installerPath."
    Write-Output "Running the installer..."
    Start-Process -FilePath $installerPath -NoNewWindow -Wait

    Write-Output "Rust installation complete. Please ensure the PATH is updated by restarting your terminal if necessary."
}
else {
    # Get the locally installed Rust version
    $localRustVersion = & rustc --version | ForEach-Object { ($_ -split " ")[1] }
    Write-Output "Rust is already installed (version $localRustVersion)."

    # Check for the latest Rust version using rustup
    Write-Output "Checking for the latest Rust version..."
    $rugstupCheckOutput = & rustup check

    # Function to parse the latest stable version from rustup check output
    function Get-LatestStableVersion($output) {
        # Find the line that starts with "stable" and extract the latest version
        $stableLine = $output | Select-String -Pattern "^stable.*" | ForEach-Object { $_.Line }
        if ($stableLine -match "-> ([0-9]+\.[0-9]+\.[0-9]+)") {
            # Extract the latest version when an update is available
            return $matches[1]
        }
        elseif ($stableLine -match ": ([0-9]+\.[0-9]+\.[0-9]+)") {
            # Extract the current version when up-to-date
            return $matches[1]
        }
        else {
            return $null
        }
    }

    # Parse the latest stable version
    $latestRustVersion = Get-LatestStableVersion $rugstupCheckOutput

    # Verify if a version was successfully parsed
    if ($latestRustVersion) {
        Write-Output "Latest Rust version: $latestRustVersion"
    }
    else {
        Write-Output "Unable to determine the latest Rust version. Please check manually."
        exit 1
    }

    # Compare installed and latest versions
    if ($localRustVersion -eq $latestRustVersion) {
        Write-Output "Rust is up-to-date."
    }
    else {
        Write-Output "Your Rust version ($localRustVersion) is outdated. Latest version is $latestRustVersion."
        $updateChoice = Read-Host "Do you want to update Rust? (y/n)"
        if ($updateChoice -eq "y") {
            rustup update
            Write-Output "Rust has been updated to the latest version."
        }
        else {
            Write-Output "Rust update skipped."
        }
    }
}

# Install Node.js using fnm
if (-not (CommandExists "node")) {
    Write-Output "Node.js is not installed. Proceeding with installation using fnm..."

    # Ensure fnm is installed
    if (-not (CommandExists "fnm")) {
        Write-Output "fnm (Fast Node Manager) is not installed. Installing fnm using cargo..."
        cargo install fnm
        Write-Output "fnm has been installed."
    }

    # Configure fnm environment
    Write-Output "Configuring fnm environment..."
    fnm env --use-on-cd | Out-String | Invoke-Expression

    # Download and install Node.js
    $nodeVersion = "22" # Specify the desired Node.js version
    Write-Output "Installing Node.js version $nodeVersion..."
    fnm use --install-if-missing $nodeVersion

    # Verify installation
    if (CommandExists "node") {
        $nodeVersionInstalled = &node --version
        Write-Output "Node.js has been successfully installed (version $nodeVersionInstalled)."
    }
    else {
        Write-Output "Node.js installation failed. Please check your fnm configuration."
    }
}
else {
    $nodeVersionInstalled = &node --version
    Write-Output "Node.js is already installed (version $nodeVersionInstalled)."
}

Write-Output "All dependencies are installed."

try {
    
    # Step 3: Configure Backend Settings
    Write-Output "Configuring backend settings..."

    # Navigate to the backend directory
    Set-Location -Path $backendDir

    # Copy default configuration files
    if (-not (Test-Path ".env")) {
        Copy-Item ".env.default" ".env"
        Write-Output "Copied .env.default to .env."
    }
    else {
        Write-Output ".env file already exists. Skipping."
    }

    if (-not (Test-Path "Rocket.toml")) {
        Copy-Item "Rocket.default.toml" "Rocket.toml"
        Write-Output "Copied Rocket.default.toml to Rocket.toml."
    }
    else {
        Write-Output "Rocket.toml file already exists. Skipping."
    }

    # Display instructions to edit the configuration
    Write-Output "Please edit the .env and Rocket.toml files to set the necessary configuration values."
    Write-Output @"
.env:
  PASSWORD: Your password for the app.
  SYNC_PATH: Directory to watch for new or modified photos.
  DISCORD_HOOK_URL: Optional Discord webhook URL for error notifications.

Rocket.toml:
  port: Default is 4000. Change if needed.
"@

    # Step 4: Build the Backend
    Write-Output "Building the backend..."
    try {
        if ($Debug) {
            Write-Output "Debug mode enabled. Running 'cargo build'..."
            cargo build
        }
        else {
            Write-Output "Running 'cargo build --release'..."
            cargo build --release
        }
        Write-Output "Backend built successfully."
    }
    catch {
        Write-Output "Failed to build the backend. Check the error logs."
        exit 1
    }

    # Step 5: Configure Frontend Settings
    Write-Output "Configuring frontend settings..."

    # Navigate to the frontend directory
    Set-Location -Path $frontendDir

    # Copy default configuration file
    if (-not (Test-Path "config.ts")) {
        Copy-Item "config.default.ts" "config.ts"
        Write-Output "Copied config.default.ts to config.ts."
    }
    else {
        Write-Output "config.ts file already exists. Skipping."
    }

    # Step 6: Build the Frontend
    Write-Output "Building the frontend..."
    try {
        npm run build
        Write-Output "Frontend built successfully."
    }
    catch {
        Write-Output "Failed to build the frontend. Check the error logs."
        exit 1
    }
}
finally {
    # Always return to the root directory
    Set-Location -Path $projectRoot
    # Write-Output "Returned to the root directory: $projectRoot"
}

Write-Output "Setup complete. Your project is ready for use!"
