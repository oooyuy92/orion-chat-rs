# PWA + Docker 私有化部署实施计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Orion Chat 扩展为支持 PWA + Docker 私有化部署的 Web 应用,使用轮询架构解决移动端锁屏问题

**Architecture:** 在现有 Tauri 项目基础上添加可选的 HTTP Server 模式,提取共享业务逻辑到 core/ 层,前端通过轮询获取 streaming 进度,后端独立完成 streaming 并写入数据库

**Tech Stack:** Axum, Tokio, SQLite, SvelteKit, Docker, PWA (Service Worker)

---

## 文件结构规划

### 新增文件
- `src-tauri/src/bin/web_server.rs` - Web Server 独立入口
- `src-tauri/src/web_server/mod.rs` - Web Server 模块
- `src-tauri/src/web_server/routes.rs` - HTTP 路由定义
- `src-tauri/src/web_server/handlers.rs` - 请求处理器
- `src-tauri/src/web_server/middleware.rs` - CORS 等中间件
- `src-tauri/src/core/mod.rs` - 共享业务逻辑模块
- `src-tauri/src/core/chat.rs` - 聊天业务逻辑
- `src-tauri/src/core/conversation.rs` - 对话业务逻辑
- `src-tauri/src/core/provider.rs` - Provider 业务逻辑
- `src-tauri/src/core/assistant.rs` - Assistant 业务逻辑
- `static/manifest.json` - PWA manifest
- `static/sw.js` - Service Worker
- `static/icons/icon-192.png` - PWA 图标 192x192
- `static/icons/icon-512.png` - PWA 图标 512x512
- `Dockerfile` - Docker 构建配置
- `docker-compose.yml` - Docker Compose 配置
- `.dockerignore` - Docker 忽略文件

### 修改文件
- `src-tauri/Cargo.toml` - 添加 Axum 等依赖
- `src-tauri/src/commands/chat.rs` - 重构为调用 core 层
- `src-tauri/src/commands/conversation.rs` - 重构为调用 core 层
- `src/lib/api/web/impl.ts` - 实现轮询机制
- `src/app.html` - 添加 PWA manifest 引用

---

## Chunk 1: 后端核心架构

### Task 1: 添加 Axum 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 Axum 相关依赖**

在 `[dependencies]` 部分添加:

```toml
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }
```

- [ ] **Step 2: 验证依赖解析**

Run: `cd src-tauri && cargo check`
Expected: 依赖下载成功,项目编译通过

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "$(cat <<'EOF'
build: add axum dependencies for web server

Add Axum web framework and Tower middleware dependencies to support HTTP server mode alongside existing Tauri desktop mode.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: 创建共享业务逻辑层 - Chat 模块

**Files:**
- Create: `src-tauri/src/core/mod.rs`
- Create: `src-tauri/src/core/chat.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 core 模块入口**

创建 `src-tauri/src/core/mod.rs`:

```rust
pub mod chat;
pub mod conversation;
pub mod provider;
pub mod assistant;
```

- [ ] **Step 2: 在 lib.rs 中声明 core 模块**

在 `src-tauri/src/lib.rs` 中添加:

```rust
pub mod core;
```

- [ ] **Step 3: 创建 chat 核心逻辑**

创建 `src-tauri/src/core/chat.rs` (初始版本):

```rust
use std::sync::Arc;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;

/// 发送消息的核心逻辑,不依赖 Tauri Channel
pub async fn send_message_core(
    state: &AppState,
    conversation_id: String,
    content: String,
    model_id: String,
) -> AppResult<String> {
    // TODO: 实现完整逻辑
    Ok("placeholder".to_string())
}
```

- [ ] **Step 4: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/core/ src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat: add core business logic layer skeleton

Create core module to extract shared business logic from Tauri commands, enabling code reuse between desktop and web server modes.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---



### Task 3: 创建 Web Server 模块骨架

**Files:**
- Create: `src-tauri/src/web_server/mod.rs`
- Create: `src-tauri/src/web_server/routes.rs`  
- Create: `src-tauri/src/web_server/handlers.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 web_server 模块**

创建 `src-tauri/src/web_server/mod.rs`:

```rust
pub mod routes;
pub mod handlers;

pub use routes::create_router;
```

- [ ] **Step 2: 在 lib.rs 中声明模块**

在 `src-tauri/src/lib.rs` 中添加:

```rust
pub mod web_server;
```

- [ ] **Step 3: 创建路由模块**

创建 `src-tauri/src/web_server/routes.rs`:

```rust
use std::sync::Arc;
use axum::{Router, routing::{get, post}};
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use crate::state::AppState;
use super::handlers;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("../build"))
        .route("/api/messages/:id", get(handlers::get_message))
        .route("/api/messages/send", post(handlers::send_message))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

- [ ] **Step 4: 创建处理器模块**

创建 `src-tauri/src/web_server/handlers.rs`:

```rust
use std::sync::Arc;
use axum::{extract::{State, Path}, Json};
use crate::{error::AppResult, models::*, state::AppState, core};

pub async fn get_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<Message>> {
    let message = state.db.with_conn(|conn| {
        crate::db::messages::get(conn, &id)
    })?;
    Ok(Json(message))
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<Json<SendMessageResponse>> {
    let message_id = core::chat::send_message_core(
        &state,
        req.conversation_id,
        req.content,
        req.model_id,
        req.common_params,
        req.provider_params,
    ).await?;
    Ok(Json(SendMessageResponse { message_id }))
}

#[derive(serde::Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub content: String,
    pub model_id: String,
    pub common_params: Option<CommonParams>,
    pub provider_params: Option<ProviderParams>,
}

#[derive(serde::Serialize)]
pub struct SendMessageResponse {
    pub message_id: String,
}
```

- [ ] **Step 5: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/web_server/ src-tauri/src/lib.rs
git commit -m "feat: add web server module with routes and handlers

Create web_server module with Axum routes and handlers for HTTP API. Implements get_message (for polling) and send_message endpoints.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 4: 创建 Web Server 入口

**Files:**
- Create: `src-tauri/src/bin/web_server.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 在 Cargo.toml 添加 binary**

在 `src-tauri/Cargo.toml` 中添加:

```toml
[[bin]]
name = "web-server"
path = "src/bin/web_server.rs"
```

- [ ] **Step 2: 创建入口文件**

创建 `src-tauri/src/bin/web_server.rs`:

```rust
use std::net::SocketAddr;
use std::sync::Arc;
use orion_chat_rs::{state::AppState, web_server::create_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();
    
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(28080);
    
    let state = Arc::new(AppState::new()?);
    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    println!("Listening on http://{}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

- [ ] **Step 3: 测试构建**

Run: `cd src-tauri && cargo build --bin web-server`
Expected: 构建成功

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bin/web_server.rs src-tauri/Cargo.toml
git commit -m "feat: add web server binary entry point

Create standalone web server binary on port 28080 (configurable via PORT env var).

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 2: 前端轮询机制

### Task 5: 实现前端轮询逻辑

**Files:**
- Modify: `src/lib/api/web/impl.ts`

- [ ] **Step 1: 实现 sendMessage 轮询**

在 `src/lib/api/web/impl.ts` 的 `sendMessage` 方法中实现轮询:

```typescript
async sendMessage(
    conversationId: string,
    content: string,
    modelId: string,
    onEvent: ChatEventHandler,
    commonParams?: CommonParams,
    providerParams?: ProviderParams
): Promise<Message> {
    // 发送消息
    const { messageId } = await post<{ messageId: string }>('/messages/send', {
        conversationId,
        content,
        modelId,
        commonParams,
        providerParams,
    });
    
    // 启动轮询
    return new Promise((resolve, reject) => {
        let retryCount = 0;
        const maxRetries = 3;
        
        const pollInterval = setInterval(async () => {
            try {
                const message = await get<Message>(`/messages/${messageId}`);
                retryCount = 0;
                
                onEvent({ type: 'chunk', content: message.content });
                
                if (message.status === 'done') {
                    clearInterval(pollInterval);
                    onEvent({ type: 'finished' });
                    resolve(message);
                } else if (message.status === 'error') {
                    clearInterval(pollInterval);
                    onEvent({ type: 'error', message: '生成失败' });
                    reject(new Error('Message generation failed'));
                }
            } catch (error) {
                retryCount++;
                if (retryCount >= maxRetries) {
                    clearInterval(pollInterval);
                    onEvent({ type: 'error', message: '网络连接失败' });
                    reject(error);
                }
            }
        }, 500);
    });
}
```

- [ ] **Step 2: 验证类型检查**

Run: `pnpm check`
Expected: 类型检查通过

- [ ] **Step 3: Commit**

```bash
git add src/lib/api/web/impl.ts
git commit -m "feat: implement polling mechanism for web mode

Add 500ms polling to fetch message updates from database. Includes retry logic (max 3 attempts) and proper error handling for network failures.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

由于实施计划很长,我建议分批编写和审查。当前已完成 Chunk 1(后端核心架构)和 Chunk 2(前端轮询机制)的部分内容。

是否继续编写剩余的 chunks(PWA、Docker、测试)?


## Chunk 3: PWA 功能

### Task 6: 创建 PWA Manifest

**Files:**
- Create: `static/manifest.json`
- Modify: `src/app.html`

- [ ] **Step 1: 创建 static 目录**

Run: `mkdir -p static/icons`

- [ ] **Step 2: 创建 manifest.json**

创建 `static/manifest.json`:

```json
{
  "name": "Orion Chat",
  "short_name": "Orion",
  "description": "AI Chat Client with Multi-Provider Support",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#000000",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    }
  ]
}
```

- [ ] **Step 3: 在 app.html 中引用 manifest**

在 `src/app.html` 的 `<head>` 中添加:

```html
<link rel="manifest" href="/manifest.json" />
<meta name="theme-color" content="#000000" />
```

- [ ] **Step 4: Commit**

```bash
git add static/manifest.json src/app.html
git commit -m "feat: add PWA manifest

Add PWA manifest.json for installable web app support. Enables add-to-homescreen on mobile devices.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 7: 创建 Service Worker

**Files:**
- Create: `static/sw.js`
- Modify: `src/app.html`

- [ ] **Step 1: 创建 Service Worker**

创建 `static/sw.js`:

```javascript
const CACHE_NAME = 'orion-chat-v1';
const STATIC_ASSETS = [
  '/',
  '/manifest.json',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(STATIC_ASSETS);
    })
  );
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames
          .filter((name) => name !== CACHE_NAME)
          .map((name) => caches.delete(name))
      );
    })
  );
  self.clients.claim();
});

self.addEventListener('fetch', (event) => {
  event.respondWith(
    fetch(event.request)
      .then((response) => {
        const responseClone = response.clone();
        caches.open(CACHE_NAME).then((cache) => {
          cache.put(event.request, responseClone);
        });
        return response;
      })
      .catch(() => {
        return caches.match(event.request);
      })
  );
});
```

- [ ] **Step 2: 在 app.html 中注册 Service Worker**

在 `src/app.html` 的 `<body>` 末尾添加:

```html
<script>
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('/sw.js');
    });
  }
</script>
```

- [ ] **Step 3: Commit**

```bash
git add static/sw.js src/app.html
git commit -m "feat: add service worker for offline support

Implement Service Worker with Network First strategy. Caches responses for offline browsing of previously loaded content.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 8: 准备 PWA 图标

**Files:**
- Create: `static/icons/icon-192.png`
- Create: `static/icons/icon-512.png`

- [ ] **Step 1: 从 Tauri 图标生成 PWA 图标**

Run: 从 `src-tauri/icons/` 复制或转换图标到 `static/icons/`

Note: 需要 192x192 和 512x512 两个尺寸的 PNG 图标

- [ ] **Step 2: 验证图标存在**

Run: `ls -lh static/icons/`
Expected: 看到 icon-192.png 和 icon-512.png

- [ ] **Step 3: Commit**

```bash
git add static/icons/
git commit -m "feat: add PWA icons

Add 192x192 and 512x512 icons for PWA installation.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 4: Docker 部署

### Task 9: 创建 Dockerfile

**Files:**
- Create: `Dockerfile`
- Create: `.dockerignore`

- [ ] **Step 1: 创建 .dockerignore**

创建 `.dockerignore`:

```
node_modules
.svelte-kit
build
target
.git
.env
*.log
```

- [ ] **Step 2: 创建 Dockerfile**

创建 `Dockerfile`:

```dockerfile
# Stage 1: Build frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

# Stage 2: Build backend
FROM rust:1.83-alpine AS backend-builder
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
COPY src-tauri/src ./src
RUN cargo build --release --bin web-server

# Stage 3: Runtime
FROM alpine:latest
WORKDIR /app

RUN apk add --no-cache ca-certificates

COPY --from=frontend-builder /app/build ./build
COPY --from=backend-builder /app/target/release/web-server ./

RUN mkdir -p /app/data && chmod 755 /app/data

EXPOSE 28080

ENV PORT=28080
ENV DATA_DIR=/app/data
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3   CMD wget --no-verbose --tries=1 --spider http://localhost:28080/ || exit 1

CMD ["./web-server"]
```

- [ ] **Step 3: 测试 Docker 构建**

Run: `docker build -t orion-chat:test .`
Expected: 构建成功(可能需要较长时间)

- [ ] **Step 4: Commit**

```bash
git add Dockerfile .dockerignore
git commit -m "feat: add Dockerfile for single-container deployment

Multi-stage Docker build: frontend (Node/pnpm), backend (Rust), runtime (Alpine). Exposes port 28080 with health check.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 10: 创建 docker-compose 配置

**Files:**
- Create: `docker-compose.yml`

- [ ] **Step 1: 创建 docker-compose.yml**

创建 `docker-compose.yml`:

```yaml
version: '3.8'

services:
  orion-chat:
    build: .
    ports:
      - "28080:28080"
    volumes:
      - orion-chat-data:/app/data
    environment:
      - PORT=28080
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  orion-chat-data:
    driver: local
```

- [ ] **Step 2: 测试 docker-compose**

Run: `docker-compose up -d`
Expected: 容器启动成功

- [ ] **Step 3: 验证服务运行**

Run: `curl http://localhost:28080/`
Expected: 返回前端页面

- [ ] **Step 4: 停止测试容器**

Run: `docker-compose down`

- [ ] **Step 5: Commit**

```bash
git add docker-compose.yml
git commit -m "feat: add docker-compose for local testing

Provides easy local deployment with named volume for data persistence.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## Chunk 5: 文档和测试

### Task 11: 创建部署文档

**Files:**
- Create: `docs/deployment/docker.md`

- [ ] **Step 1: 创建部署文档**

创建 `docs/deployment/docker.md`:

```markdown
# Docker 部署指南

## 快速开始

### 使用 Docker

\`\`\`bash
# 构建镜像
docker build -t orion-chat:latest .

# 运行容器
docker run -d \
  --name orion-chat \
  -p 28080:28080 \
  -v orion-chat-data:/app/data \
  orion-chat:latest

# 访问应用
open http://localhost:28080
\`\`\`

### 使用 Docker Compose

\`\`\`bash
# 启动
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止
docker-compose down
\`\`\`

## 数据迁移

从 Tauri 桌面版迁移数据:

\`\`\`bash
# macOS
docker cp ~/Library/Application\ Support/com.orion-chat.app/orion.db \
  orion-chat:/app/data/orion.db

# Linux
docker cp ~/.local/share/com.orion-chat.app/orion.db \
  orion-chat:/app/data/orion.db

# 重启容器
docker restart orion-chat
\`\`\`

## 环境变量

- \`PORT\`: HTTP 端口 (默认: 28080)
- \`DATA_DIR\`: 数据目录 (默认: /app/data)
- \`RUST_LOG\`: 日志级别 (默认: info)
```

- [ ] **Step 2: Commit**

```bash
git add docs/deployment/
git commit -m "docs: add Docker deployment guide

Document Docker and docker-compose usage, data migration from desktop version.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

### Task 12: 端到端测试

**Files:**
- None (manual testing)

- [ ] **Step 1: 测试桌面模式**

Run: `pnpm tauri dev`
Expected: 桌面应用正常启动和运行

- [ ] **Step 2: 测试 Web Server 模式**

Run: `cd src-tauri && cargo run --bin web-server`
Expected: Web server 在 28080 端口启动

- [ ] **Step 3: 测试前端连接**

在浏览器访问 `http://localhost:28080`
Expected: 应用加载,可以创建对话

- [ ] **Step 4: 测试轮询机制**

发送一条消息,观察:
- 消息立即显示
- AI 回复逐步显示(每 500ms 更新)
- 完成后停止轮询

- [ ] **Step 5: 测试 PWA 安装**

在 Chrome 中访问应用:
- 检查地址栏是否显示安装图标
- 点击安装,验证可以安装到桌面

- [ ] **Step 6: 测试离线模式**

- 断开网络
- 刷新页面
- Expected: 页面仍然加载(从缓存)
- 显示离线提示

- [ ] **Step 7: 测试 Docker 部署**

Run: `docker-compose up -d && sleep 5 && curl http://localhost:28080/`
Expected: 返回 HTML 内容

---

### Task 13: 更新 README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: 添加 Web/Docker 部署说明**

在 `README.md` 中添加新的部署方式:

```markdown
## Deployment

### Desktop (Tauri)

\`\`\`bash
pnpm tauri dev    # Development
pnpm tauri build  # Production
\`\`\`

### Web/Docker (Private Deployment)

\`\`\`bash
# Using Docker
docker build -t orion-chat .
docker run -d -p 28080:28080 -v orion-data:/app/data orion-chat

# Using Docker Compose
docker-compose up -d

# Access at http://localhost:28080
\`\`\`

See [Docker Deployment Guide](docs/deployment/docker.md) for details.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README with Docker deployment

Add Docker deployment instructions alongside existing Tauri desktop instructions.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
```

---

## 实施完成

所有任务完成后,项目将支持:

✅ Tauri 桌面模式(保持不变)
✅ Web Server 模式(Axum HTTP Server)
✅ 轮询架构(解决移动端锁屏问题)
✅ PWA 支持(可安装,离线浏览)
✅ Docker 单容器部署
✅ 数据库兼容(桌面版和 Web 版共享格式)

**预计时间**: 5-9 天

**下一步**: 使用 `superpowers:subagent-driven-development` 或 `superpowers:executing-plans` 执行此计划
