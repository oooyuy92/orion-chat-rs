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
