# Orion Chat

<div align="center">

🚀 **Lightweight AI Chat Client - 1/10 Memory Usage of Electron Apps**

Built with Tauri v2 + Rust, supports Desktop, Web, and PWA deployment

[Quick Start](#-quick-start) · [Features](#-core-features) · [Deployment](docs/deployment.md) · [Architecture](#️-tech-stack)

</div>

---

## ⚡ Why Choose Orion Chat

### Comparison with Mainstream Clients

| Feature | Orion Chat | Cherry Studio | ChatBox | ChatGPT Desktop | Open WebUI | LobeChat |
|---------|-----------|---------------|---------|-----------------|------------|----------|
| **Tech Stack** | Tauri + Rust | Electron | Electron | Electron | Web | Web |
| **Memory Usage** | ~50-80MB | ~500-800MB | ~300-500MB | ~400-600MB | ~200-400MB | ~150-300MB |
| **Package Size** | ~15MB | ~150-200MB | ~100-150MB | ~120MB | N/A | N/A |
| **Startup Speed** | < 1s | 3-5s | 3-5s | 3-5s | N/A | N/A |
| **Desktop App** | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| **Self-hosted** | ✅ Docker | ❌ | ❌ | ❌ | ✅ Docker | ✅ Docker |
| **PWA Support** | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Mobile Friendly** | ✅ Polling | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Unified 3-Platform** | ✅ Desktop+Web+PWA | ❌ | ❌ | ❌ | ❌ | ❌ |

### Core Advantages

- 🪶 **Ultra Lightweight**: Built on Tauri, 1/10 memory usage of Electron apps
- 🚀 **Instant Startup**: Rust native performance, < 1 second startup
- 🔒 **Fully Private**: One-click Docker self-hosting, complete data control
- 📱 **Unified 3-Platform**: Desktop + Web + PWA, one codebase for all
- 🔄 **Smart Polling**: Solves mobile lock screen issues, continuous background generation
- 🎨 **Modern UI**: Svelte 5 + shadcn, smooth experience

---

## 📊 Performance Comparison

### Memory Usage Benchmark

```
Idle State:
- Orion Chat (Tauri):  ~50MB   ⚡
- Cherry Studio:       ~500MB
- ChatBox:             ~300MB
- ChatGPT Desktop:     ~400MB

Running 3 Conversations:
- Orion Chat (Tauri):  ~80MB   ⚡
- Cherry Studio:       ~800MB
- ChatBox:             ~500MB
- ChatGPT Desktop:     ~600MB
```

### Why So Lightweight?

- **Tauri**: Uses system native WebView, no Chromium bundled
- **Rust Backend**: Zero runtime overhead, memory safe
- **SQLite**: Lightweight local storage, no extra database process

---

## 🚀 Quick Start

### Method 1: Docker Deployment (Recommended, Self-hosted)

```bash
# Clone repository
git clone https://github.com/oooyuy92/orion-chat-rs.git
cd orion-chat-rs

# One-click deployment
docker compose up -d

# Access application
open http://localhost:28080
```

### Method 2: Desktop Application

#### Download Installer (Recommended)

📥 [Download Latest Release](https://github.com/oooyuy92/orion-chat-rs/releases/latest)

Supported Platforms:
- **Windows**: `.msi` or `.exe` installer
- **macOS**: `.dmg` or `.app` bundle
- **Linux**: `.deb`, `.AppImage` or `.rpm` package

#### Local Development

```bash
# Install dependencies
pnpm install

# Start development mode
pnpm tauri dev

# Build desktop application
pnpm tauri build
```

### Method 3: PWA Installation

1. Visit the deployed web version in browser
2. Click the "Install" icon in the address bar
3. Add to home screen, supports offline use

---

## 💡 Core Features

### Conversation Management

- ✅ Multi-conversation parallel management
- ✅ Message version switching and regeneration
- ✅ Full-text search (SQLite FTS5)
- ✅ Conversation export (Markdown/JSON)
- ✅ Auto conversation rename and compression

### Multi-Provider Support

- ✅ OpenAI-compatible APIs
- ✅ Anthropic Claude
- ✅ Google Gemini
- ✅ Ollama (local models)
- ✅ Custom API endpoints

### Smart Features

- ✅ Streaming responses
- ✅ Message soft delete and recovery
- ✅ Assistant presets
- ✅ Model parameter configuration
- ✅ Local data backup

### Deployment Options

- ✅ Desktop application (Tauri)
- ✅ Web deployment (Docker)
- ✅ PWA installation
- ✅ Fully private

---

## 🏗️ Tech Stack

### Why Tauri?

- 🪶 **Lightweight**: No Chromium bundled, uses system WebView
- ⚡ **High Performance**: Rust native performance, zero runtime overhead
- 🔒 **Secure**: Rust memory safety guarantee
- 📦 **Small Size**: Installer only ~15MB

### Technology Stack

**Frontend**
- SvelteKit + Svelte 5
- Tailwind CSS 4
- shadcn-svelte components

**Desktop**
- Tauri v2
- System native WebView

**Backend**
- Rust + Tokio
- Axum HTTP Server
- Reqwest (HTTP client)

**Storage**
- SQLite + FTS5 full-text search
- Local data persistence

---

## 📦 Deployment Guide

### Docker Deployment

```bash
# Using docker-compose (recommended)
docker compose up -d

# Or using docker command
docker build -t orion-chat .
docker run -d -p 28080:28080 -v orion-data:/data orion-chat
```

### Environment Variables

```bash
DATABASE_PATH=/data/orion.db  # Database path
DATA_DIR=/data                # Data directory
STATIC_DIR=/app/static        # Static files directory
PORT=28080                    # Service port
```

### Data Persistence

- Using Named Volume: `orion-data`
- Database location: `/data/orion.db`
- Data persists after container restart

Detailed deployment documentation: [docs/deployment.md](docs/deployment.md)

---

## 🛠️ Development Guide

### Requirements

- Node.js 20+
- pnpm
- Rust toolchain
- Tauri prerequisites

### Local Development

```bash
# Install dependencies
pnpm install

# Start development server
pnpm tauri dev

# Type checking
pnpm check

# Build desktop application
pnpm tauri build

# Build web version
pnpm build
```

### Project Structure

```
orion-chat-rs/
├── src/                      # SvelteKit frontend
│   ├── lib/
│   │   ├── components/       # UI components
│   │   ├── stores/           # State management
│   │   └── api/              # API wrappers
│   └── routes/               # Route pages
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── commands/         # Tauri commands
│   │   ├── core/             # Shared business logic
│   │   ├── db/               # Database layer
│   │   ├── providers/        # Provider integrations
│   │   ├── web_server/       # Web Server module
│   │   └── bin/              # Binary entry points
│   └── Cargo.toml
├── static/                   # Static assets
│   ├── manifest.json         # PWA manifest
│   └── sw.js                 # Service Worker
├── Dockerfile                # Docker build
├── docker-compose.yml        # Docker Compose config
└── docs/                     # Documentation
    └── deployment.md         # Deployment docs
```

---

## 🗺️ Roadmap

### ✅ Completed

- [x] Multi-provider support (OpenAI/Anthropic/Gemini/Ollama)
- [x] Desktop application (Tauri v2)
- [x] Conversation management and search
- [x] Message version control
- [x] Docker deployment
- [x] PWA support
- [x] Web Server mode
- [x] Polling architecture

### 🚧 In Progress

- [ ] Mobile optimization
- [ ] More provider support
- [ ] Plugin system

### 📋 Planned

- [ ] Multi-language support
- [ ] Theme customization
- [ ] Data synchronization
- [ ] Team collaboration features

---

## 📄 License

MIT License

---

## 🙏 Acknowledgments

This project uses the following excellent open source projects:

- [Tauri](https://tauri.app/) - Build lightweight desktop applications
- [SvelteKit](https://kit.svelte.dev/) - Modern frontend framework
- [Rust](https://www.rust-lang.org/) - High-performance backend language
- [shadcn-svelte](https://www.shadcn-svelte.com/) - UI component library

---

<div align="center">

**If this project helps you, please give it a ⭐ Star!**

Made with ❤️ by the Orion Chat team

</div>

---
---

# 中文版 / Chinese Version

---

# Orion Chat

<div align="center">

🚀 **轻量级 AI 聊天客户端 - 内存占用仅为 Electron 应用的 1/10**

基于 Tauri v2 + Rust 构建,支持桌面、Web、PWA 三端部署

[快速开始](#-快速开始) · [功能特性](#-核心功能) · [部署文档](docs/deployment.md) · [技术架构](#️-技术架构)

</div>

---

## ⚡ 为什么选择 Orion Chat

### 与主流客户端对比

| 特性 | Orion Chat | Cherry Studio | ChatBox | ChatGPT Desktop | Open WebUI | LobeChat |
|------|-----------|---------------|---------|-----------------|------------|----------|
| **技术栈** | Tauri + Rust | Electron | Electron | Electron | Web | Web |
| **内存占用** | ~50-80MB | ~500-800MB | ~300-500MB | ~400-600MB | ~200-400MB | ~150-300MB |
| **安装包大小** | ~15MB | ~150-200MB | ~100-150MB | ~120MB | N/A | N/A |
| **启动速度** | < 1s | 3-5s | 3-5s | 3-5s | N/A | N/A |
| **桌面应用** | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| **自部署** | ✅ Docker | ❌ | ❌ | ❌ | ✅ Docker | ✅ Docker |
| **PWA 支持** | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| **移动友好** | ✅ 轮询架构 | ❌ | ❌ | ❌ | ✅ | ✅ |
| **三端统一** | ✅ 桌面+Web+PWA | ❌ | ❌ | ❌ | ❌ | ❌ |

### 核心优势

- 🪶 **极致轻量**: 基于 Tauri,内存占用仅为 Electron 应用的 1/10
- 🚀 **秒速启动**: Rust 原生性能,启动时间 < 1 秒
- 🔒 **完全私有**: Docker 一键自部署,数据完全自主可控
- 📱 **三端统一**: 桌面 + Web + PWA,一套代码多端部署
- 🔄 **智能轮询**: 解决移动端锁屏问题,后台持续生成
- 🎨 **现代 UI**: Svelte 5 + shadcn,流畅体验

---

## 📊 性能对比

### 内存占用实测

```
空闲状态:
- Orion Chat (Tauri):  ~50MB   ⚡
- Cherry Studio:       ~500MB
- ChatBox:             ~300MB
- ChatGPT Desktop:     ~400MB

运行 3 个对话:
- Orion Chat (Tauri):  ~80MB   ⚡
- Cherry Studio:       ~800MB
- ChatBox:             ~500MB
- ChatGPT Desktop:     ~600MB
```

### 为什么这么轻量?

- **Tauri**: 使用系统原生 WebView,不打包 Chromium
- **Rust 后端**: 零运行时开销,内存安全
- **SQLite**: 轻量级本地存储,无需额外数据库进程

---

## 🚀 快速开始

### 方式 1: Docker 部署 (推荐,支持自部署)

```bash
# 克隆仓库
git clone https://github.com/oooyuy92/orion-chat-rs.git
cd orion-chat-rs

# 一键部署
docker compose up -d

# 访问应用
open http://localhost:28080
```

### 方式 2: 桌面应用

#### 下载安装包 (推荐)

📥 [下载最新版本](https://github.com/oooyuy92/orion-chat-rs/releases/latest)

支持平台:
- **Windows**: `.msi` 或 `.exe` 安装包
- **macOS**: `.dmg` 或 `.app` 应用包
- **Linux**: `.deb`, `.AppImage` 或 `.rpm` 安装包

#### 本地开发

```bash
# 安装依赖
pnpm install

# 启动开发模式
pnpm tauri dev

# 构建桌面应用
pnpm tauri build
```

### 方式 3: PWA 安装

1. 浏览器访问部署的 Web 版本
2. 点击地址栏右侧的"安装"图标
3. 添加到主屏幕,支持离线使用

---

## 💡 核心功能

### 对话管理

- ✅ 多对话并行管理
- ✅ 消息版本切换和重新生成
- ✅ 全文搜索 (SQLite FTS5)
- ✅ 对话导出 (Markdown/JSON)
- ✅ 对话自动重命名和压缩

### 多 Provider 支持

- ✅ OpenAI-compatible APIs
- ✅ Anthropic Claude
- ✅ Google Gemini
- ✅ Ollama (本地模型)
- ✅ 自定义 API 端点

### 智能特性

- ✅ 流式响应
- ✅ 消息软删除和恢复
- ✅ Assistant 预设
- ✅ 模型参数配置
- ✅ 本地数据备份

### 部署选项

- ✅ 桌面应用 (Tauri)
- ✅ Web 部署 (Docker)
- ✅ PWA 安装
- ✅ 完全私有化

---

## 🏗️ 技术架构

### 为什么选择 Tauri?

- 🪶 **轻量级**: 不打包 Chromium,使用系统 WebView
- ⚡ **高性能**: Rust 原生性能,零运行时开销
- 🔒 **安全**: Rust 内存安全保证
- 📦 **小体积**: 安装包仅 ~15MB

### 技术栈

**前端**
- SvelteKit + Svelte 5
- Tailwind CSS 4
- shadcn-svelte 组件

**桌面端**
- Tauri v2
- 系统原生 WebView

**后端**
- Rust + Tokio
- Axum HTTP Server
- Reqwest (HTTP 客户端)

**存储**
- SQLite + FTS5 全文搜索
- 本地数据持久化

---

## 📦 部署指南

### Docker 部署

```bash
# 使用 docker-compose (推荐)
docker compose up -d

# 或使用 docker 命令
docker build -t orion-chat .
docker run -d -p 28080:28080 -v orion-data:/data orion-chat
```

### 环境变量

```bash
DATABASE_PATH=/data/orion.db  # 数据库路径
DATA_DIR=/data                # 数据目录
STATIC_DIR=/app/static        # 静态文件目录
PORT=28080                    # 服务端口
```

### 数据持久化

- 使用 Named Volume: `orion-data`
- 数据库位置: `/data/orion.db`
- 数据会在容器重启后保留

详细部署文档: [docs/deployment.md](docs/deployment.md)

---

## 🛠️ 开发指南

### 环境要求

- Node.js 20+
- pnpm
- Rust toolchain
- Tauri prerequisites

### 本地开发

```bash
# 安装依赖
pnpm install

# 启动开发服务器
pnpm tauri dev

# 类型检查
pnpm check

# 构建桌面应用
pnpm tauri build

# 构建 Web 版本
pnpm build
```

### 项目结构

```
orion-chat-rs/
├── src/                      # SvelteKit 前端
│   ├── lib/
│   │   ├── components/       # UI 组件
│   │   ├── stores/           # 状态管理
│   │   └── api/              # API 封装
│   └── routes/               # 路由页面
├── src-tauri/                # Rust 后端
│   ├── src/
│   │   ├── commands/         # Tauri 命令
│   │   ├── core/             # 共享业务逻辑
│   │   ├── db/               # 数据库层
│   │   ├── providers/        # Provider 集成
│   │   ├── web_server/       # Web Server 模块
│   │   └── bin/              # 二进制入口
│   └── Cargo.toml
├── static/                   # 静态资源
│   ├── manifest.json         # PWA manifest
│   └── sw.js                 # Service Worker
├── Dockerfile                # Docker 构建
├── docker-compose.yml        # Docker Compose 配置
└── docs/                     # 文档
    └── deployment.md         # 部署文档
```

---

## 🗺️ Roadmap

### ✅ 已完成

- [x] 多 Provider 支持 (OpenAI/Anthropic/Gemini/Ollama)
- [x] 桌面应用 (Tauri v2)
- [x] 对话管理和搜索
- [x] 消息版本控制
- [x] Docker 部署
- [x] PWA 支持
- [x] Web Server 模式
- [x] 轮询架构

### 🚧 进行中

- [ ] 移动端优化
- [ ] 更多 Provider 支持
- [ ] 插件系统

### 📋 计划中

- [ ] 多语言支持
- [ ] 主题定制
- [ ] 数据同步
- [ ] 团队协作功能

---

## 📄 License

MIT License

---

## 🙏 致谢

本项目使用了以下优秀的开源项目:

- [Tauri](https://tauri.app/) - 构建轻量级桌面应用
- [SvelteKit](https://kit.svelte.dev/) - 现代化前端框架
- [Rust](https://www.rust-lang.org/) - 高性能后端语言
- [shadcn-svelte](https://www.shadcn-svelte.com/) - UI 组件库

---

<div align="center">

**如果这个项目对你有帮助,请给个 ⭐ Star!**

Made with ❤️ by the Orion Chat team

</div>

