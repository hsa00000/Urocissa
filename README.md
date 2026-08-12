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

[https://demo.photoserver.tw](https://demo.photoserver.tw)

This demo showcases the typical usage of Urocissa, allowing you to experience its core features and user interface.

### One-Million-Photo Demo

[https://demo-million.photoserver.tw](https://demo-million.photoserver.tw)

This demo demonstrates Urocissa's ability to manage 1,000,000 photos, showcasing the power and scalability of Urocissa. Since I don't have access to a million unique images, the photos in this demo are replaced with placeholders.

Both demos are currently in read-only mode, and uploading files or editing tags is not permitted at this time.

## Advantages

- **Blazing Fast Performance**: Index photos with a pure Rust crate. Instantly serve, search, and filter one million photos in under a second using an in-memory cached database.

- **Memory Efficient**: Even with the entire database cached in memory, both the standard demo and the one-million-photo demo can run seamlessly on a single server with just 4 GB of RAM.

- **Infinite Photo Stream**: Experience endless scrolling without pagination. No lazy loading needed. Urocissa uses advanced virtual scrolling to serve one million photos, overcoming the DOM height limit of 33,554,400px (see [TanStack/virtual#616](https://github.com/TanStack/virtual/issues/616)).

- **Instant Data Search**: Use boolean operators such as 'and', 'or', or 'not' to search your data instantly. Find examples of search queries [here](docs/SEARCH.md).

## Limitations

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
| Rotation Images            | ✅     |
| Discovery                  | ⏳     |
| Multi-User Support         | ❌     |
| Object/Face Recognition    | ❌     |
| Geolocation/Map            | ❌     |
| Android App                | ❌     |
| External Libraries         | ❌     |
| Existing Folders           | ❌     |

## Memory Usage Estimate

Urocissa uses an in-memory cached database to ensure instant access and blazing-fast search. Based on a local V6 performance benchmark, the following table estimates the RAM needed to handle large numbers of photos:

| Photo Count | Estimated RAM Usage |
| ----------- | ------------------- |
| 1 million   | ~0.49 GiB           |
| 2 million   | ~0.98 GiB           |
| 3 million   | ~1.47 GiB           |
| 4 million   | ~1.96 GiB           |
| 5 million   | ~2.45 GiB           |
| 6 million   | ~2.95 GiB           |
| 8 million   | ~3.93 GiB           |
| 10 million  | ~4.91 GiB           |

The 1-million-record run reached a peak RSS (resident memory) of 502.6 MiB, including a full-selection edit and write-behind drain. The benchmark uses metadata-only records, so media files are not included. Larger values are linear extrapolations; actual usage may vary with allocator behavior, indexing options, and memory reuse patterns.

## Quick Setup

- **Windows Users**: You can download the installation file directly from the [Releases](https://github.com/hsa00000/urocissa/releases) page.

- **Linux Users**: To instantly set up and try Urocissa using Docker, follow these steps:

### Quick Setup with Docker

1. **Clone the Repository**

   Start by cloning the Urocissa repository from GitHub:

   ```bash
   git clone https://github.com/hsa00000/urocissa.git
   ```

2. **Navigate to the Project Directory**

   Enter the newly created `urocissa` directory:

   ```bash
   cd urocissa
   ```

3. **Run the Launch Script**

   Execute the `run_urocissa_docker.sh` script to launch Urocissa:

   ```bash
   bash run_urocissa_docker.sh
   ```

This script will launch Urocissa. You can access the app using the following link:

[http://127.0.0.1:5673](http://127.0.0.1:5673)

If you want to change the default port or configure a password, refer to the [Configuration Guide](docs/CONFIG.md).

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
   bash run_urocissa_docker.sh
   ```

   This will update and launch Urocissa.

## Build from Source (Without Using Docker)

If you prefer to build and install Urocissa from source, follow the relevant guide for your operating system:

- **Linux Users**: Refer to the instructions in [this guide](docs/LINUX.md).
- **Windows Users**: Check out the instructions in [this guide](docs/WINDOWS.md).
