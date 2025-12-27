## Steps to Set Up and Use the App (Windows Version)

Follow these instructions to set up and run the Urocissa app on a Windows machine.

### 1. Clone the Repository

First, ensure that you have Git for Windows installed. Then clone the repository using:

```bash
git clone https://github.com/hsa00000/Urocissa.git
```

This will create a folder called `./Urocissa`.

---

### 2. Install Dependencies

Make sure the following software is installed on your system:

- **ffmpeg**: Download FFmpeg from the official [FFmpeg website](https://ffmpeg.org/download.html). Extract the downloaded folder, and add the `bin` directory to your system's PATH environment variable.

- **Rust**: Install Rust using the [official installer](https://www.rust-lang.org/tools/install) for Windows.

- **Node.js (with npm)**: Download and install Node.js from the official [Node.js website](https://nodejs.org). Make sure npm is included in the installation.

---

### 3. Build the Frontend

In the `frontend` directory, run:

```bash
npm run build
```

---

### 4. Configure Backend Settings

1. Navigate to the backend directory:

   ```bash
   cd ./Urocissa/backend
   ```

2. Copy the default config file and fill in the necessary settings:

   ```bash
   copy .env.default .env
   copy Rocket.default.toml Rocket.toml
   ```

   **.env:**

   ```env
   PASSWORD=password
   SYNC_PATH=
   DISCORD_HOOK_URL=
   ```

   _Explanation:_

   - `PASSWORD`: Your password for the app.
   - `SYNC_PATH`: A comma-separated list of directories that the app will monitor for new or modified photos. For example: `SYNC_PATH=./some/relative/path,/some/absolute/path`.
   - `DISCORD_HOOK_URL`: (Optional) Fill in your Discord webhook URL to receive error notifications.

   **Rocket.toml:**

   - `port`: Default is `5673`. You can change this to your desired port number.

---

### 5. Run the Application

Navigate to the `backend` directory and run the following command to start the app:

```bash
cargo run --release
```

You can now access the app via [http://127.0.0.1:5673](http://127.0.0.1:5673) or [http://127.0.0.1](http://127.0.0.1):\<your_port> if you configured a custom port in Rocket.toml.

---

## Update

### 1. Pull the Latest Changes from the Repository

Navigate to the project directory and pull the latest updates:

```bash
git pull
```

### 2. Rebuild

If using Docker, follow these steps:

1. Pull the latest Docker image:

   ```bash
   docker pull hsa00000/urocissa:latest
   ```

2. Run the Docker script:

   ```bash
   bash run_urocissa_docker.sh
   ```

This will update and start the updated app.

---

If you are not using Docker and prefer to build from source, follow these manual steps to update:

### Rebuild the Frontend

1. Navigate to the `frontend` directory:

   ```bash
   cd ./Urocissa/frontend
   ```

2. Build the frontend:

   ```bash
   npm run build
   ```

### Rebuild the Backend

1. Navigate to the `backend` directory:

   ```bash
   cd ./Urocissa/backend
   ```

2. Build and run the backend using Cargo:

   ```bash
   cargo run --release
   ```

After following these steps, your Urocissa app will be updated to the latest version.
