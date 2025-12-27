![螢幕擷取畫面 2024-10-17 213036](https://github.com/user-attachments/assets/b8de7937-1916-4b73-9c31-667c7eb1a23d)

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

- **Instant Data Search**: Use boolean operators such as 'and', 'or', or 'not' to search your data instantly. Find examples of search queries [here](docs/SEARCH.md).

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
| Shareable Albums           | ✅     |
| Basic Editing              | 🛠️     |
| Discovery                  | ⏳     |
| Multi-User Support         | ❌     |
| Object/Face Recognition    | ❌     |
| Geolocation/Map            | ❌     |
| Android App                | ❌     |
| External Libraries         | ❌     |
| Existing Folders           | ❌     |

## Memory Usage Estimate

Urocissa uses an in-memory cached database to ensure instant access and blazing-fast search. Based on real-world measurements, the following table estimates the RAM needed to handle large numbers of photos:

| Photo Count | Estimated RAM Usage |
| ----------- | ------------------- |
| 1 million   | ~1.2 GiB            |
| 2 million   | ~2.4 GiB            |
| 3 million   | ~3.6 GiB            |
| 4 million   | ~4.8 GiB            |
| 5 million   | ~6.0 GiB            |
| 6 million   | ~7.2 GiB            |
| 8 million   | ~9.6 GiB            |
| 10 million  | ~12 GiB             |

These values are based on actual runtime RSS (resident memory) usage of the `urocissa` process, measured during full data generation. In-memory usage may vary slightly depending on runtime allocator behavior, indexing options, or memory reuse patterns, but the scaling is approximately linear.

## Quick Setup

To instantly set up and try Urocissa using Docker on Linux, follow these steps:

### Quick Setup with Docker

1. **Clone the Repository**

   Start by cloning the Urocissa repository from GitHub:

   ```bash
   git clone https://github.com/hsa00000/Urocissa.git
   ```

2. **Navigate to the Project Directory**

   Enter the newly created `Urocissa` directory:

   ```bash
   cd Urocissa
   ```

3. **Run the Launch Script**

   Execute the `run_urocissa_docker.sh` script to launch Urocissa:

   ```bash
   bash docker/run_urocissa_docker.sh
   ```

This script will launch Urocissa. You can access the app using the following link:

[http://127.0.0.1:5673](http://127.0.0.1:5673)

The default login password is `password`. If you want to change the default port or password, refer to the [Configuration Guide](docs/LINUX.md#3-configure-backend-settings).

### Quick Update with Docker

1. Navigate to the project directory and pull the latest updates:

   ```bash
   git pull
   ```

1. Pull the latest Docker image:

   ```bash
   docker pull hsa00000/urocissa:latest
   ```

1. Run the Docker script:

   ```bash
   bash docker/run_urocissa_docker.sh
   ```

   This will update and launch Urocissa.

## Build from Source (Without Using Docker)

If you prefer to build and install Urocissa from source, follow the relevant guide for your operating system:

- **Linux Users**: Refer to the instructions in [this guide](docs/LINUX.md).
- **Windows Users**: Check out the instructions in [this guide](docs/WINDOWS.md).
