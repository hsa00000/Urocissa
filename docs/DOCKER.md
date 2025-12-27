# Docker 部署指南 | Docker Deployment Guide

[English](#english) | [繁體中文](#繁體中文)

---

## English

### Overview

Urocissa provides Docker images for easy deployment. This guide explains how to run Urocissa using Docker Compose.

### Prerequisites

- Docker Engine 20.10+
- Docker Compose v2.0+

### Quick Start

#### 1. Clone the Repository

```bash
git clone https://github.com/hsa00000/Urocissa.git
cd Urocissa
```

#### 2. Create Configuration Files

Copy the default configuration files:

```bash
# Backend configuration
cp backend/config.default.json backend/config.json
cp backend/Rocket.default.toml backend/Rocket.toml
cp backend/.env.default backend/.env
```

#### 3. Configure Environment Variables

Edit `backend/.env` to configure your installation:

```dotenv
# Required: Set a secure password for the web interface
PASSWORD=your_secure_password

# Required: Paths to your photo directories (comma-separated)
# These paths will be scanned for photos
SYNC_PATH=/path/to/your/photos,/another/path/to/photos

# Optional: Discord webhook URL for notifications
DISCORD_HOOK_URL=
```

#### 4. Configure Photo Library Volumes

Edit `docker/docker-compose.yml` and add your photo directories under the `volumes` section:

```yaml
volumes:
  # ... existing volumes ...

  # Add your photo directories here:
  - /path/to/your/photos:/path/to/your/photos:ro
  - /another/path/to/photos:/another/path/to/photos:ro
```

> **Important:** The container path should match the host path exactly because `SYNC_PATH` references these locations.

#### 5. Start Urocissa

```bash
cd docker
docker compose up -d
```

#### 6. Access the Web Interface

Open your browser and navigate to: `http://localhost:5673`

---

### Configuration Reference

#### Environment Variables (.env)

| Variable           | Required | Description                                      |
| ------------------ | -------- | ------------------------------------------------ |
| `PASSWORD`         | Yes      | Password for web interface authentication        |
| `SYNC_PATH`        | Yes      | Comma-separated list of paths to scan for photos |
| `DISCORD_HOOK_URL` | No       | Discord webhook URL for notifications            |

#### Rocket.toml (Web Server)

| Setting   | Default   | Description        |
| --------- | --------- | ------------------ |
| `address` | `0.0.0.0` | IP address to bind |
| `port`    | `5673`    | Port number        |

#### config.json (Application)

| Setting        | Default | Description              |
| -------------- | ------- | ------------------------ |
| `readOnlyMode` | `false` | Enable read-only mode    |
| `disableImg`   | `false` | Disable image processing |

---

### Advanced Usage

#### Use Development Image

```bash
UROCISSA_IMAGE=hsa00000/urocissa:dev docker compose up -d
```

#### Custom Port

```bash
UROCISSA_PORT=8080 docker compose up -d
```

#### Build Locally

```bash
docker compose up -d --build
```

#### Build with Debug Profile

```bash
BUILD_TYPE=debug docker compose up -d --build
```

#### View Logs

```bash
docker compose logs -f urocissa
```

#### Stop and Remove Container

```bash
docker compose down
```

---

### Available Docker Images

| Tag                              | Description                        |
| -------------------------------- | ---------------------------------- |
| `hsa00000/urocissa:latest`       | Latest stable release (multi-arch) |
| `hsa00000/urocissa:dev`          | Development build (multi-arch)     |
| `hsa00000/urocissa:latest-amd64` | Latest stable for AMD64            |
| `hsa00000/urocissa:latest-arm64` | Latest stable for ARM64            |
| `hsa00000/urocissa:dev-amd64`    | Development build for AMD64        |
| `hsa00000/urocissa:dev-arm64`    | Development build for ARM64        |

---

### Troubleshooting

#### Container exits immediately

Check if all required configuration files exist:

- `backend/.env`
- `backend/Rocket.toml`
- `backend/config.json`

#### Photos not appearing

1. Verify `SYNC_PATH` in `.env` matches your volume mounts
2. Ensure paths are mounted with correct permissions
3. Check container logs: `docker compose logs urocissa`

#### Port already in use

Change the port in `backend/Rocket.toml` and update the compose file:

```bash
UROCISSA_PORT=8080 docker compose up -d
```

---

## 繁體中文

### 概述

Urocissa 提供 Docker 映像檔以便於部署。本指南說明如何使用 Docker Compose 執行 Urocissa。

### 前置需求

- Docker Engine 20.10+
- Docker Compose v2.0+

### 快速開始

#### 1. 複製儲存庫

```bash
git clone https://github.com/hsa00000/Urocissa.git
cd Urocissa
```

#### 2. 建立設定檔

複製預設設定檔：

```bash
# 後端設定
cp backend/config.default.json backend/config.json
cp backend/Rocket.default.toml backend/Rocket.toml
cp backend/.env.default backend/.env
```

#### 3. 設定環境變數

編輯 `backend/.env` 來配置您的安裝：

```dotenv
# 必填：設定網頁介面的安全密碼
PASSWORD=你的安全密碼

# 必填：照片目錄的路徑（以逗號分隔）
# 這些路徑將被掃描以尋找照片
SYNC_PATH=/path/to/your/photos,/another/path/to/photos

# 選填：Discord webhook URL 用於通知
DISCORD_HOOK_URL=
```

#### 4. 設定照片庫磁碟區

編輯 `docker/docker-compose.yml`，在 `volumes` 區段下新增您的照片目錄：

```yaml
volumes:
  # ... 現有的磁碟區 ...

  # 在此新增您的照片目錄：
  - /path/to/your/photos:/path/to/your/photos:ro
  - /another/path/to/photos:/another/path/to/photos:ro
```

> **重要：** 容器內的路徑必須與主機路徑完全相同，因為 `SYNC_PATH` 會參照這些位置。

#### 5. 啟動 Urocissa

```bash
cd docker
docker compose up -d
```

#### 6. 存取網頁介面

開啟瀏覽器並前往：`http://localhost:5673`

---

### 設定參考

#### 環境變數 (.env)

| 變數               | 必填 | 說明                               |
| ------------------ | ---- | ---------------------------------- |
| `PASSWORD`         | 是   | 網頁介面的認證密碼                 |
| `SYNC_PATH`        | 是   | 要掃描照片的路徑清單（以逗號分隔） |
| `DISCORD_HOOK_URL` | 否   | Discord webhook URL 用於通知       |

#### Rocket.toml（網頁伺服器）

| 設定      | 預設值    | 說明           |
| --------- | --------- | -------------- |
| `address` | `0.0.0.0` | 綁定的 IP 位址 |
| `port`    | `5673`    | 連接埠號       |

#### config.json（應用程式）

| 設定           | 預設值  | 說明         |
| -------------- | ------- | ------------ |
| `readOnlyMode` | `false` | 啟用唯讀模式 |
| `disableImg`   | `false` | 停用圖片處理 |

---

### 進階用法

#### 使用開發版映像檔

```bash
UROCISSA_IMAGE=hsa00000/urocissa:dev docker compose up -d
```

#### 自訂連接埠

```bash
UROCISSA_PORT=8080 docker compose up -d
```

#### 本地建置

```bash
docker compose up -d --build
```

#### 使用 Debug 設定檔建置

```bash
BUILD_TYPE=debug docker compose up -d --build
```

#### 檢視日誌

```bash
docker compose logs -f urocissa
```

#### 停止並移除容器

```bash
docker compose down
```

---

### 可用的 Docker 映像檔

| 標籤                             | 說明                   |
| -------------------------------- | ---------------------- |
| `hsa00000/urocissa:latest`       | 最新穩定版本（多架構） |
| `hsa00000/urocissa:dev`          | 開發版本（多架構）     |
| `hsa00000/urocissa:latest-amd64` | AMD64 的最新穩定版     |
| `hsa00000/urocissa:latest-arm64` | ARM64 的最新穩定版     |
| `hsa00000/urocissa:dev-amd64`    | AMD64 的開發版本       |
| `hsa00000/urocissa:dev-arm64`    | ARM64 的開發版本       |

---

### 疑難排解

#### 容器立即退出

檢查所有必要的設定檔是否存在：

- `backend/.env`
- `backend/Rocket.toml`
- `backend/config.json`

#### 照片未顯示

1. 確認 `.env` 中的 `SYNC_PATH` 與您的磁碟區掛載相符
2. 確保路徑以正確的權限掛載
3. 檢查容器日誌：`docker compose logs urocissa`

#### 連接埠已被使用

在 `backend/Rocket.toml` 中更改連接埠，並更新 compose 檔案：

```bash
UROCISSA_PORT=8080 docker compose up -d
```
