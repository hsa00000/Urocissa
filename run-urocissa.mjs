import { execSync } from "child_process";
import { resolve } from "path";
import { chdir } from "process";

// Resolve the current script's directory
const projectRoot = resolve();
const backendDir = resolve(projectRoot, "gallery-backend");

// Helper function to gracefully handle errors
function handleError(message) {
    console.error(message);
    chdir(projectRoot);
    console.log(`Returned to the root directory: ${projectRoot}`);
    process.exit(1);
}

// Navigate to the backend directory
try {
    console.log(`Navigating to the backend directory: ${backendDir}`);
    chdir(backendDir);
} catch (error) {
    handleError("Failed to navigate to the backend directory.");
}

// Execute the backend
try {
    console.log("Starting the backend using 'cargo run --release'...");
    execSync("cargo run --release", { stdio: "inherit" });
    console.log("Backend is running successfully.");
} catch (error) {
    handleError("Failed to run the backend. Check the error logs.");
}

// Always return to the root directory
try {
    chdir(projectRoot);
    console.log(`Returned to the root directory: ${projectRoot}`);
} catch (error) {
    console.error("Failed to return to the root directory.");
    process.exit(1);
}
