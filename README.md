# Urocissa

Urocissa is a self-hosted gallery designed to serve massive collections, capable of handling millions of images and videos. It is built using Rust and Vue.

## Table of Contents

- [Motivation](#motivation)
- [Demo](#demo)
- [Advantages](#advantages)
- [Limitations](#limitations)
- [Steps to Set Up and Use the App](#steps-to-set-up-and-use-the-app)
- [Update](#update)

## Motivation

The goal of this project is to efficiently serve one million photos on a 4 GB RAM server, providing smooth scrubbable scrolling, infinite photo streams, and instant search and selection, without waiting for the entire database to load in the browser.

## Demo

You can explore the features of Urocissa through the following demos:

### Standard Demo

[https://demo.photoserver.tw](https://demo.photoserver.tw)\
**Password:** `password`

This demo showcases the typical usage of Urocissa, allowing you to experience its core features and user interface.

### One-Million-Photo Demo

[https://demo-million.photoserver.tw](https://demo-million.photoserver.tw)\
**Password:** `password`

This demo demonstrates Urocissa's ability to manage 1,000,000 photos, showcasing the power and scalability of Urocissa. Since I don't have access to a million unique images, the photos in this demo are replaced with placeholders.

Both demos are currently in read-only mode, and uploading files or editing tags is not permitted at this time.

## Advantages

- **Blazing Fast Performance**: Index photos with a pure Rust crate. Instantly serve, search, and filter one million photos in under a second using an in-memory cached database.

- **Memory Efficient**: Even with the entire database cached in memory, both the standard demo and the one-million-photo demo can run seamlessly on a single server with just 4 GB of RAM.

- **Infinite Photo Stream**: Experience endless scrolling without pagination. No lazy loading needed. Urocissa uses advanced virtual scrolling to serve one million photos, overcoming the DOM height limit of 33,554,400px (see [TanStack/virtual#616](https://github.com/TanStack/virtual/issues/616)).

- **Instant Data Search**: Use boolean operators such as 'and', 'or', or 'not' to search your data instantly. Find examples of search queries [here](https://github.com/hsa00000/Urocissa/blob/main/SEARCH.md).

## Limitations

**Early Stage Development**: The app is still in its very early development phase. Many features are incomplete, and there are no automated tests. Additionally, Urocissa is currently optimized for Chrome and Firefox on Windows and Android, but it may encounter issues for browsers on iOS or Linux. The detailed features can be seen in this table:

| Feature                    | Status |
| -------------------------- | ------ |
| Upload Videos and Photos   | ✅     |
| Auto Backup Folders        | ✅     |
| Download Photos and Videos | ✅     |
| EXIF Data                  | ✅     |
| User-Defined Tags          | ✅     |
| Duplicate Handling         | ✅     |
| Instant Select All         | ✅     |
| Find in Timeline           | ✅     |
| Responsive Layout          | ✅     |
| Docker Installation        | ✅     |
| Shareable Albums           | 🛠️     |
| Basic Editing              | ⏳     |
| Multi-User Support         | ⏳     |
| Discovery                  | ⏳     |
| Object/Face Recognition    | ❌     |
| Geolocation/Map            | ❌     |
| Android App                | ❌     |
| External Libraries         | ❌     |
| Existing Folders           | ❌     |

## Steps to Set Up and Use the App

To set up the app, please follow the appropriate guide for your operating system:

- **Linux Users**: Follow the instructions in [this guide](https://github.com/hsa00000/Urocissa/blob/main/LINUX.md).
- **Windows Users**: Follow the instructions in [this guide](https://github.com/hsa00000/Urocissa/blob/main/WINDOWS.md).

To quickly set up and try Urocissa using Docker, follow these steps:

> **Note**: Docker provides a quick and convenient setup but is still in an early development stage, resulting in lower efficiency. It is best suited for testing or trial purposes. For optimal performance, building from source is recommended.

### Quick Setup with Docker

1. **Clone the Repository**

   First, clone the Urocissa repository from GitHub:

   ```bash
   git clone https://github.com/hsa00000/Urocissa.git
   ```

2. **Navigate to the Project Directory**

   Move into the `Urocissa` directory that was just created:

   ```bash
   cd Urocissa
   ```

3. **Run the Setup Script**

   Run the `bash run_urocissa_docker.sh` file to install and start Urocissa:

   ```bash
   bash run_urocissa_docker.sh
   ```

   This script will handle all necessary installations and start the application, allowing you to get Urocissa up and running in one step.

## Update

### 1. Pull the Latest Changes from the Repository

Navigate to the project directory and pull the latest updates:

```bash
git pull

```

If using Docker, simply run the script

```bash
bash run_urocissa_docker.sh
```

Otherwise, follow these manual steps to update:

---

### 2. Rebuild the Frontend

1. Navigate to the `gallery-frontend` directory:

   ```bash
   cd ./Urocissa/gallery-frontend
   ```

2. Build the frontend:

   ```bash
   npm run build
   ```

---

### 3. Rebuild the Backend

1. Navigate to the `gallery-backend` directory:

   ```bash
   cd ./Urocissa/gallery-backend
   ```

2. Build and run the backend using Cargo:

   ```bash
   cargo run --release
   ```

---

After following these steps, your Urocissa app will be updated to the latest version.
