import os from "os";
import { execSync } from "child_process";
import { existsSync, readdirSync, copyFileSync, rmSync } from "fs";
import { resolve, join } from "path";

const args = process.argv.slice(2);
const debug = args.includes("--debug");
const update = args.includes("--update");

// Define project directories
const projectRoot = resolve();
const backendDir = join(projectRoot, "gallery-backend");
const frontendDir = join(projectRoot, "gallery-frontend");

// Helper function to execute shell commands
function runCommand(command, options = {}) {
    try {
        execSync(command, { stdio: "inherit", ...options });
    } catch (error) {
        console.error(`Error executing command: ${command}`);
        process.exit(1);
    }
}

// Helper function to check if a command exists
function commandExists(command) {
    try {
        execSync(`${command}`, { stdio: "pipe" });
        return true;
    } catch {
        return false;
    }
}

// Perform update tasks
if (update) {
    console.log("Update mode enabled. Performing update tasks...");

    // Step 1: Pull the latest changes
    console.log("Pulling the latest changes from the repository...");
    runCommand("git pull", { cwd: projectRoot });

    // Step 2: Rebuild the backend
    console.log("Rebuilding the backend...");
    const buildCommand = debug ? "cargo build" : "cargo build --release";
    runCommand(buildCommand, { cwd: backendDir });

    // Step 3: Rebuild the frontend
    console.log("Rebuilding the frontend...");
    runCommand("npm run build", { cwd: frontendDir });

    console.log("Update completed successfully!");
    process.exit(0);
}

// Install FFmpeg
if (!commandExists("ffmpeg -version")) {
    console.log("FFmpeg is not installed. Installing FFmpeg...");

    if (os.platform() === "win32") {
        // Windows installation: Download and extract FFmpeg
        const ffmpegUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
        const ffmpegArchiveName = "ffmpeg.zip";

        console.log(`Downloading FFmpeg from ${ffmpegUrl}...`);
        runCommand(`curl -L -o ${ffmpegArchiveName} ${ffmpegUrl}`);

        console.log("Extracting FFmpeg...");
        runCommand(`tar -xf ${ffmpegArchiveName}`);

        // Locate the extracted directory
        const extractedDir = readdirSync(".").find(dir => dir.startsWith("ffmpeg-") && dir.endsWith("-essentials_build"));

        if (extractedDir) {
            const ffmpegPath = resolve(join(extractedDir, "bin", "ffmpeg.exe"));
            if (existsSync(ffmpegPath)) {
                copyFileSync(ffmpegPath, "ffmpeg.exe");
                console.log("FFmpeg setup complete!");

                // Cleanup: Remove zip file and extracted folder
                console.log("Cleaning up temporary files...");
                rmSync(ffmpegArchiveName); // Delete the zip file
                rmSync(extractedDir, { recursive: true, force: true }); // Delete the extracted folder
                console.log("Cleanup complete!");
            } else {
                console.error("Failed to locate FFmpeg binary in the extracted directory.");
                process.exit(1);
            }
        } else {
            console.error("Failed to locate the extracted FFmpeg directory. Ensure the archive was extracted properly.");
            process.exit(1);
        }
    } else if (os.platform() === "linux") {
        // Linux installation: Use apt
        console.log("Installing FFmpeg using apt...");
        runCommand("sudo apt update");
        runCommand("sudo apt install -y ffmpeg");
        console.log("FFmpeg installed successfully.");
    } else {
        console.error(`Unsupported operating system: ${os.platform()}`);
        process.exit(1);
    }
} else {
    console.log("FFmpeg is already installed.");
}


// Install Rust
if (!commandExists("rustup --version")) {
    console.log("Rust is not installed. Installing Rust...");

    if (os.platform() === "win32") {
        // Windows installation
        const rustInstallerUrl = process.arch === "x64"
            ? "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
            : "https://static.rust-lang.org/rustup/dist/i686-pc-windows-msvc/rustup-init.exe";
        const installerPath = "rustup-init.exe";
        console.log(`Downloading Rust installer from ${rustInstallerUrl}...`);
        runCommand(`curl -L -o ${installerPath} ${rustInstallerUrl}`);
        console.log("Running Rust installer...");
        runCommand(installerPath);
    } else if (os.platform() === "linux") {
        // Linux installation
        console.log("Installing Rust using rustup script...");
        runCommand(`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`);
        console.log("Rust installed successfully. You may need to restart your terminal to use Rust.");
    } else {
        console.error(`Unsupported operating system: ${os.platform()}`);
        process.exit(1);
    }
} else {
    console.log("Rust is already installed.");
}

// Configure and build the backend
console.log("Configuring backend settings...");
if (!existsSync(join(backendDir, ".env"))) {
    copyFileSync(join(backendDir, ".env.default"), join(backendDir, ".env"));
}
if (!existsSync(join(backendDir, "Rocket.toml"))) {
    copyFileSync(join(backendDir, "Rocket.default.toml"), join(backendDir, "Rocket.toml"));
}
runCommand(debug ? "cargo build" : "cargo build --release", { cwd: backendDir });

// Configure and build the frontend
console.log("Configuring frontend settings...");
if (!existsSync(join(frontendDir, "config.ts"))) {
    copyFileSync(join(frontendDir, "config.default.ts"), join(frontendDir, "config.ts"));
}
runCommand("npm run build", { cwd: frontendDir });

console.log("Setup complete. Your project is ready for use!");
