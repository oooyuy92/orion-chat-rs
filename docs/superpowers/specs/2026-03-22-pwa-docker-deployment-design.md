# Orion Chat PWA + Docker 私有化部署设计方案

**设计时间**: 2026-03-22
**目标**: 将 Orion Chat 从桌面应用扩展为支持 PWA + Docker 私有化部署的 Web 应用

---

## 1. 设计目标

### 1.1 用户目标
- **桌面端用户**: 保持现有 Tauri 体验,无感知变化
- **Web 端用户**: 通过 Docker 一键部署,浏览器访问
- **移动端用户**: PWA 安装后获得类原生体验,支持离线浏览历史对话

### 1.2 业务目标
- 统一代码库,业务逻辑只写一次
- 降低部署门槛(Docker 单容器部署)
- 扩大用户覆盖(支持移动设备)
- 保持桌面端性能和功能优势
- 解决移动端锁屏时 streaming 中断的问题

### 1.3 技术约束
- 单容器 Docker 部署
- Named Volume 数据持久化
- 无认证模式(适合个人私有化部署)
- 离线浏览支持(PWA)
- 轮询架构(解决锁屏问题)
- 使用 Axum 作为 Web 框架
- 数据库兼容(SQLite 格式一致)

---

## 2. 整体架构

### 2.1 项目结构

```
orion-chat-rs/
├── src/                          # SvelteKit 前端 (已有)
│   ├── lib/
│   │   ├── api/
│   │   │   ├── index.ts         # 统一 API 入口 (已有)
│   │   │   ├── platform.ts      # 平台检测 (已有)
│   │   │   ├── tauri/impl.ts    # Tauri 实现 (已有)
│   │   │   └── web/impl.ts      # Web 实现 (已有)
│   │   └── ...
│   └── routes/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri 桌面应用入口 (已有)
│   │   ├── lib.rs               # 共享库 (已有)
│   │   ├── bin/
│   │   │   └── web_server.rs    # 新增: Web Server 入口
│   │   ├── web_server/          # 新增: Web Server 模块
│   │   │   ├── mod.rs
│   │   │   ├── routes.rs        # HTTP 路由
│   │   │   ├── handlers.rs      # 请求处理器
│   │   │   └── middleware.rs    # CORS 等中间件
│   │   ├── core/                # 新增: 共享业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs          # 从 commands/chat.rs 提取
│   │   │   ├── conversation.rs  # 从 commands/conversation.rs 提取
│   │   │   └── ...
│   │   ├── commands/            # 保留: Tauri commands
│   │   ├── db/                  # 保留: 数据库层
│   │   └── providers/           # 保留: Provider 集成
│   └── Cargo.toml
├── static/                       # 新增: 静态资源
│   ├── manifest.json            # PWA manifest
│   ├── sw.js                    # Service Worker
│   └── icons/                   # PWA 图标
├── Dockerfile                    # 新增: Docker 构建
└── docker-compose.yml           # 新增: 本地测试用
```

### 2.2 运行模式

**Tauri 桌面模式** (保持不变):
```bash
pnpm tauri dev    # 开发
pnpm tauri build  # 构建
```

**Web Server 模式** (新增):
```bash
cargo run --bin web-server                    # 开发
docker run -v orion-data:/app/data orion-chat # 生产
```

### 2.3 代码共享策略

将现有的 `src-tauri/src/commands/` 中的业务逻辑提取到 `src-tauri/src/core/`,供两种模式共用:

- **Tauri 模式**: `commands/` → `core/` → 数据库
- **Web 模式**: `web_server/handlers/` → `core/` → 数据库

这样可以确保两种模式的行为完全一致,避免代码重复。

---

## 3. 核心组件设计

### 3.1 Web Server 入口

**文件**: `src-tauri/src/bin/web_server.rs`

独立的二进制入口,不依赖 Tauri:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::init();

    // 读取配置
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(28080);

    // 初始化应用状态(数据库、providers 等)
    let state = AppState::new()?;

    // 启动 Axum server
    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### 3.2 共享业务逻辑层

**目录**: `src-tauri/src/core/`

将现有的 `commands/` 中的业务逻辑提取为纯函数,不依赖 Tauri:

```rust
// src-tauri/src/core/chat.rs
pub async fn send_message_core(
    state: &AppState,
    conversation_id: String,
    content: String,
    model_id: String,
) -> AppResult<String> {
    // 1. 创建 user message
    let user_msg_id = create_user_message(state, &conversation_id, &content)?;

    // 2. 创建 assistant message placeholder
    let assistant_msg_id = create_assistant_placeholder(
        state, &conversation_id, &model_id
    )?;

    // 3. 启动后台任务进行 streaming
    let state_clone = state.clone();
    tokio::spawn(async move {
        stream_to_database(state_clone, assistant_msg_id, ...).await
    });

    // 4. 返回 assistant message_id
    Ok(assistant_msg_id)
}

// 后台 streaming 任务
async fn stream_to_database(
    state: Arc<AppState>,
    message_id: String,
    // ...
) {
    // 独立完成 streaming,直接写入数据库
    // 不需要通过 channel 推送给前端
}
```

### 3.3 HTTP 路由层

**文件**: `src-tauri/src/web_server/routes.rs`

```rust
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // 静态文件服务
        .nest_service("/", ServeDir::new("../build"))

        // API 路由
        .route("/api/conversations", get(list_conversations))
        .route("/api/conversations", post(create_conversation))
        .route("/api/messages/:id", get(get_message))  // 轮询端点
        .route("/api/messages/send", post(send_message))
        .route("/api/providers", get(list_providers))
        .route("/api/models", get(list_models))
        // ... 其他路由

        // 中间件
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())

        // 共享状态
        .with_state(state)
}
```

### 3.4 轮询机制

#### 前端实现

**文件**: `src/lib/api/web/impl.ts`

```typescript
async sendMessage(conversationId, content, modelId, onEvent) {
    // 1. 发送消息,获取 message_id
    const { messageId } = await post('/messages/send', {
        conversationId, content, modelId
    });

    // 2. 启动轮询
    let retryCount = 0;
    const maxRetries = 3;

    const pollInterval = setInterval(async () => {
        try {
            const message = await get(`/messages/${messageId}`);
            retryCount = 0;  // 重置重试计数

            // 触发事件回调
            onEvent({ type: 'chunk', content: message.content });

            // 检查是否完成
            if (message.status === 'done' || message.status === 'error') {
                clearInterval(pollInterval);
                onEvent({ type: 'finished' });
            }
        } catch (error) {
            retryCount++;

            if (retryCount >= maxRetries) {
                clearInterval(pollInterval);
                onEvent({
                    type: 'error',
                    message: '网络连接失败,请检查网络后刷新页面'
                });
            }
        }
    }, 500);  // 500ms 轮询间隔

    return messageId;
}
```

#### 后端实现

**文件**: `src-tauri/src/web_server/handlers.rs`

```rust
async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>> {
    // 调用共享业务逻辑
    let message_id = core::chat::send_message_core(
        &state,
        req.conversation_id,
        req.content,
        req.model_id,
    ).await?;

    Ok(Json(SendMessageResponse { message_id }))
}

async fn get_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Message>> {
    // 从数据库读取最新状态
    let message = state.db.with_conn(|conn| {
        db::messages::get(conn, &id)
    })?;

    Ok(Json(message))
}
```

---

## 4. 配置与部署

### 4.1 端口和环境变量

**默认端口**: 28080

**环境变量**:
```bash
PORT=28080             # HTTP 端口
DATA_DIR=/app/data     # 数据目录(Docker 内部路径)
RUST_LOG=info          # 日志级别
```

### 4.2 Docker 配置

**Dockerfile** (多阶段构建):

```dockerfile
# Stage 1: 构建前端
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

# Stage 2: 构建后端
FROM rust:1.83-alpine AS backend-builder
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
COPY src-tauri/src ./src
RUN cargo build --release --bin web-server

# Stage 3: 运行时镜像
FROM alpine:latest
WORKDIR /app

# 安装运行时依赖
RUN apk add --no-cache ca-certificates

# 复制构建产物
COPY --from=frontend-builder /app/build ./static
COPY --from=backend-builder /app/target/release/web-server ./

# 创建数据目录
RUN mkdir -p /app/data && chmod 755 /app/data

# 暴露端口
EXPOSE 28080

# 设置环境变量
ENV PORT=28080
ENV DATA_DIR=/app/data
ENV RUST_LOG=info

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:28080/ || exit 1

# 启动服务
CMD ["./web-server"]
```

**docker-compose.yml** (本地测试用):

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

### 4.3 使用方式

**构建镜像**:
```bash
docker build -t orion-chat:latest .
```

**运行容器**:
```bash
docker run -d \
  --name orion-chat \
  -p 28080:28080 \
  -v orion-chat-data:/app/data \
  orion-chat:latest

# 访问: http://localhost:28080
```

**数据迁移** (从 Tauri 桌面版):
```bash
# 1. 找到桌面版数据库位置
# macOS: ~/Library/Application Support/com.orion-chat.app/orion.db
# Linux: ~/.local/share/com.orion-chat.app/orion.db
# Windows: %APPDATA%\com.orion-chat.app\orion.db

# 2. 复制到 Docker volume
docker cp ~/Library/Application\ Support/com.orion-chat.app/orion.db \
  orion-chat:/app/data/orion.db

# 3. 重启容器
docker restart orion-chat
```

### 4.4 PWA 配置

**manifest.json** (`static/manifest.json`):

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

**Service Worker** (`static/sw.js`):

```javascript
const CACHE_NAME = 'orion-chat-v1';
const STATIC_ASSETS = [
  '/',
  '/manifest.json',
];

// 安装时缓存静态资源
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(STATIC_ASSETS);
    })
  );
});

// 离线浏览策略: Network First, fallback to Cache
self.addEventListener('fetch', (event) => {
  event.respondWith(
    fetch(event.request)
      .then((response) => {
        // 缓存成功的响应
        const responseClone = response.clone();
        caches.open(CACHE_NAME).then((cache) => {
          cache.put(event.request, responseClone);
        });
        return response;
      })
      .catch(() => {
        // 网络失败,从缓存读取
        return caches.match(event.request);
      })
  );
});
```

---

## 5. 数据流设计

### 5.1 消息发送流程 (轮询架构)

```
用户输入消息
    ↓
前端: POST /api/messages/send
    ↓
后端: send_message handler
    ├─ 创建 user message (写入数据库)
    ├─ 创建 assistant message placeholder (status: streaming)
    └─ 启动后台任务 (tokio::spawn)
    ↓
返回 { message_id } 给前端
    ↓
前端: 启动轮询 (每 500ms)
    ↓
GET /api/messages/:id
    ↓
后端: 从数据库读取最新状态
    ↓
返回 { content, status, ... }
    ↓
前端: 更新 UI
    ↓
检查 status
    ├─ streaming → 继续轮询
    ├─ done → 停止轮询,显示完成
    └─ error → 停止轮询,显示错误

后台任务 (并行执行):
    ↓
调用 Provider API (streaming)
    ↓
每收到一个 chunk
    ↓
累积内容并写入数据库
    ↓
UPDATE messages SET content = ?, status = 'streaming'
    ↓
继续接收下一个 chunk
    ↓
streaming 完成
    ↓
UPDATE messages SET content = ?, status = 'done', token_count = ?
```

### 5.2 离线浏览流程 (PWA)

```
用户打开应用
    ↓
Service Worker 拦截请求
    ↓
检查网络状态
    ├─ 在线 → 从服务器获取最新数据
    │         └─ 缓存到 Cache Storage
    └─ 离线 → 从 Cache Storage 读取
              ├─ 静态资源 (HTML/CSS/JS)
              ├─ API 响应 (对话列表、消息)
              └─ 显示离线提示 (无法发送新消息)
```

### 5.3 数据库写入策略

为了平衡性能和实时性,采用以下策略:

**Streaming 期间**:
- 每收到 1-2 个 chunk 就写入一次数据库(约 200ms 间隔或 500 字符)
- 使用事务批量写入,减少 I/O

**完成时**:
- 写入最终内容、token 统计、状态

**代码示例**:
```rust
let mut buffer = String::new();
let mut last_write = Instant::now();

while let Some(chunk) = stream.next().await {
    buffer.push_str(&chunk.content);

    // 每 200ms 或累积 500 字符写入一次
    if last_write.elapsed() > Duration::from_millis(200)
        || buffer.len() > 500 {

        state.db.with_conn(|conn| {
            db::messages::update_content(
                conn, &message_id, &buffer, None, None, None
            )
        })?;

        last_write = Instant::now();
    }
}
```

### 5.4 状态同步

**前端状态管理**:
- 使用 Svelte stores 管理对话和消息状态
- 轮询更新时,只更新变化的消息
- 避免全量刷新,减少 UI 闪烁

**后端状态管理**:
- 无状态设计,所有状态存储在数据库
- 每个请求独立处理,不依赖内存状态
- 后台任务通过 message_id 关联数据库记录

---

## 6. 错误处理与边界情况

### 6.1 网络错误处理

**前端轮询中断**:
- 实现重试机制(最多 3 次)
- 超过重试次数后显示错误提示
- 建议用户检查网络后刷新页面

### 6.2 Provider API 错误处理

**后端 streaming 失败**:
- 捕获 Provider API 错误
- 将错误信息写入数据库(message.status = 'error')
- 记录详细日志供排查
- 前端轮询时获取错误状态并显示

### 6.3 数据库错误处理

**连接失败**:
- 启动时检查数据库连接
- 失败时打印清晰的错误信息
- 提示用户检查 DATA_DIR 环境变量和权限

**写入失败**:
- 使用事务确保原子性
- 失败时回滚,不影响其他操作
- 记录详细错误日志供排查

### 6.4 Docker 容器错误处理

**数据目录权限问题**:
- Dockerfile 中确保数据目录有正确的权限
- 提供健康检查机制

**容器健康检查**:
- 每 30 秒检查一次服务是否响应
- 连续 3 次失败后标记为不健康

### 6.5 PWA 离线状态处理

**离线提示**:
- 监听 online/offline 事件
- 显示离线横幅提示用户
- 禁用发送消息功能
- 允许浏览已加载的对话历史

### 6.6 边界情况

**并发请求**:
- 数据库使用 SQLite 的 WAL 模式,支持并发读
- 写操作通过连接池串行化,避免冲突

**长时间 streaming**:
- 设置合理的超时时间(如 5 分钟)
- 超时后标记为错误状态,允许用户重试

**数据库文件损坏**:
- 启动时检查数据库完整性
- 提供备份恢复机制

**磁盘空间不足**:
- 定期清理缓存和临时文件
- 提供磁盘使用情况查询 API

---

## 7. 实施计划概要

### 7.1 阶段划分

**阶段 1: 核心架构** (2-3 天)
- 提取共享业务逻辑到 `core/`
- 实现 Web Server 入口和路由
- 实现轮询机制

**阶段 2: Docker 部署** (1-2 天)
- 编写 Dockerfile
- 配置环境变量和数据持久化
- 测试 Docker 构建和运行

**阶段 3: PWA 功能** (1-2 天)
- 添加 manifest.json
- 实现 Service Worker
- 测试离线浏览功能

**阶段 4: 测试和优化** (1-2 天)
- 端到端测试
- 性能优化
- 文档编写

### 7.2 预期时间

总计: 5-9 天

---

## 8. 技术决策总结

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 部署模式 | 单容器 Docker | 部署简单,适合个人私有化 |
| 数据持久化 | Named Volume | 用户无需关心数据位置 |
| 认证方式 | 无认证 | 适合个人本地/内网部署 |
| PWA 功能 | 离线浏览 | 平衡实现复杂度和用户体验 |
| Streaming 架构 | 轮询 | 彻底解决锁屏问题 |
| Web 框架 | Axum | 与现有 Tokio 栈一致 |
| 数据迁移 | 数据库兼容 | 实现成本为零 |
| 默认端口 | 28080 | 避免与常见服务冲突 |

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 轮询延迟影响体验 | 中 | 500ms 间隔足够流畅,可根据反馈调整 |
| 数据库写入频繁 | 低 | SQLite 性能足够,使用 WAL 模式优化 |
| Docker 镜像体积大 | 低 | 多阶段构建,只包含必要文件 |
| PWA 兼容性问题 | 中 | 提供降级方案,不支持 PWA 时仍可正常使用 |

---

## 10. 后续优化方向

1. **性能优化**: 实现消息分页加载,减少初始加载时间
2. **功能增强**: 支持完全离线模式(离线发送,网络恢复后同步)
3. **部署优化**: 提供预构建的 Docker 镜像,减少用户构建时间
4. **监控告警**: 添加 Prometheus metrics,方便监控服务状态
5. **多用户支持**: 可选的多用户模式,支持团队协作场景

---

**设计完成日期**: 2026-03-22
