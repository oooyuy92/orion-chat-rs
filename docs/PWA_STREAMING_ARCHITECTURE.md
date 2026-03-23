# PWA 流式架构设计文档

## 1. 核心需求

### 1.1 功能需求
- ✅ **流式显示**：AI回复逐字显示（像ChatGPT）
- ✅ **锁屏继续**：手机锁屏后后台继续生成
- ✅ **断线恢复**：刷新页面后看到完整内容
- ✅ **多端同步**：多个设备可以同时观看生成进度

### 1.2 技术约束
- 使用 Server-Sent Events (SSE)
- 后台任务与SSE连接解耦
- 内容实时持久化到数据库

## 2. 架构设计

### 2.1 整体流程

```
用户发送消息
    ↓
POST /api/conversations/:id/messages
    ↓
创建用户消息 + 创建助手消息（status=Streaming）
    ↓
启动后台任务（tokio::spawn）
    ↓
返回 { messageId, status: "streaming" }
    ↓
前端立即建立SSE连接
    ↓
GET /api/messages/:messageId/stream
    ↓
SSE订阅后台任务的进度通道
    ↓
后台任务开始生成
    ↓
每收到一个chunk → 发送到进度通道 + 追加到数据库
    ↓
SSE实时推送chunk给前端
    ↓
前端逐字显示（流式效果）✓
    ↓
[如果连接断开]
    ↓
后台任务继续运行 ✓
    ↓
[用户刷新页面]
    ↓
GET /api/messages/:messageId
    ↓
从数据库读取已生成内容 + 重新建立SSE连接
    ↓
继续接收新chunk（如果还在生成）
```

### 2.2 核心组件

#### 2.2.1 AppState 扩展

```rust
pub struct AppState {
    pub db: Database,
    pub data_dir: PathBuf,
    pub providers: Mutex<HashMap<String, Arc<dyn Provider>>>,
    pub cancel_tokens: Mutex<HashMap<String, watch::Sender<bool>>>,
    pub proxy_mode: Mutex<String>,

    // 新增：后台任务管理
    pub generation_tasks: Mutex<HashMap<String, GenerationTask>>,
}

pub struct GenerationTask {
    pub message_id: String,
    pub conversation_id: String,
    pub task_handle: tokio::task::JoinHandle<()>,
    pub progress_tx: watch::Sender<StreamChunk>,
    pub started_at: String,
}

#[derive(Clone, Debug)]
pub enum StreamChunk {
    Content(String),      // 内容chunk
    Done(TokenStats),     // 生成完成
    Error(String),        // 错误
}
```

#### 2.2.2 后台任务系统

```rust
// 启动后台生成任务
pub async fn spawn_generation_task(
    state: Arc<AppState>,
    conversation_id: String,
    message_id: String,
    model_id: String,
    common_params: Option<CommonParams>,
    provider_params: Option<ProviderParams>,
) -> Result<(), AppError> {
    // 创建进度通道
    let (progress_tx, _progress_rx) = watch::channel(StreamChunk::Content(String::new()));

    // 克隆必要的数据
    let state_clone = state.clone();
    let progress_tx_clone = progress_tx.clone();

    // 启动独立任务
    let task_handle = tokio::spawn(async move {
        // 1. 解析provider
        let (provider, provider_type, request_model) =
            resolve_provider(&state_clone, &model_id).await?;

        // 2. 加载历史消息
        let history = state_clone.db.with_conn(|conn| {
            db::messages::list_by_conversation(conn, &conversation_id)
        })?;

        // 3. 构建请求
        let request = build_chat_request(&state_clone, &history, &request_model, ...);

        // 4. 流式调用provider
        let mut accumulated_content = String::new();

        let result = provider.stream_chat(request, message_id.clone(),
            |chunk| {
                // 每收到一个chunk
                accumulated_content.push_str(&chunk);

                // 1. 发送到进度通道（SSE会接收）
                let _ = progress_tx_clone.send(StreamChunk::Content(chunk.clone()));

                // 2. 实时写入数据库
                let _ = state_clone.db.with_conn(|conn| {
                    db::messages::update_content(
                        conn,
                        &message_id,
                        &accumulated_content,
                        None, None, None
                    )
                });
            },
            cancel_rx
        ).await;

        // 5. 完成后更新状态
        match result {
            Ok(stats) => {
                let _ = progress_tx_clone.send(StreamChunk::Done(stats));
                let _ = state_clone.db.with_conn(|conn| {
                    db::messages::set_done(conn, &message_id, stats.prompt_tokens, stats.completion_tokens)
                });
            }
            Err(e) => {
                let _ = progress_tx_clone.send(StreamChunk::Error(e.to_string()));
                let _ = state_clone.db.with_conn(|conn| {
                    db::messages::set_error(conn, &message_id, &e.to_string())
                });
            }
        }

        // 6. 清理任务
        state_clone.generation_tasks.lock().await.remove(&message_id);
    });

    // 注册任务
    state.generation_tasks.lock().await.insert(
        message_id.clone(),
        GenerationTask {
            message_id,
            conversation_id,
            task_handle,
            progress_tx,
            started_at: chrono_now(),
        }
    );

    Ok(())
}
```

#### 2.2.3 SSE Handler

```rust
// SSE流式端点
pub async fn stream_message(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // 1. 检查任务是否存在
    let progress_rx = {
        let tasks = state.generation_tasks.lock().await;
        match tasks.get(&message_id) {
            Some(task) => task.progress_tx.subscribe(),
            None => {
                // 任务不存在，可能已完成或不存在
                return Sse::new(futures::stream::once(async {
                    Ok(Event::default()
                        .event("error")
                        .data("Task not found or already completed"))
                }));
            }
        }
    };

    // 2. 创建SSE流
    let stream = async_stream::stream! {
        let mut rx = progress_rx;

        // 发送开始事件
        yield Ok(Event::default()
            .event("started")
            .data(serde_json::json!({ "messageId": message_id }).to_string()));

        // 持续接收并推送chunk
        while rx.changed().await.is_ok() {
            let chunk = rx.borrow().clone();

            match chunk {
                StreamChunk::Content(content) => {
                    yield Ok(Event::default()
                        .event("chunk")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "content": content
                        }).to_string()));
                }
                StreamChunk::Done(stats) => {
                    yield Ok(Event::default()
                        .event("done")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "promptTokens": stats.prompt_tokens,
                            "completionTokens": stats.completion_tokens
                        }).to_string()));
                    break;
                }
                StreamChunk::Error(error) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(serde_json::json!({
                            "messageId": message_id,
                            "message": error
                        }).to_string()));
                    break;
                }
            }
        }
    };

    Sse::new(stream)
}
```

### 2.3 API 端点设计

#### 2.3.1 发送消息（启动任务）

```
POST /api/conversations/:id/messages
Content-Type: application/json

{
  "content": "用户消息内容",
  "modelId": "model-uuid",
  "commonParams": { ... },
  "providerParams": { ... }
}

Response 200:
{
  "messageId": "msg-uuid",
  "status": "streaming",
  "createdAt": "2026-03-23T10:00:00"
}
```

#### 2.3.2 订阅流式进度（SSE）

```
GET /api/messages/:messageId/stream
Accept: text/event-stream

Response (SSE stream):

event: started
data: {"messageId":"msg-uuid"}

event: chunk
data: {"messageId":"msg-uuid","content":"Hello"}

event: chunk
data: {"messageId":"msg-uuid","content":" world"}

event: done
data: {"messageId":"msg-uuid","promptTokens":10,"completionTokens":20}
```

#### 2.3.3 获取消息状态

```
GET /api/messages/:messageId

Response 200:
{
  "id": "msg-uuid",
  "conversationId": "conv-uuid",
  "role": "assistant",
  "content": "Hello world",  // 已生成的内容
  "status": "streaming",     // streaming | done | error
  "modelId": "model-uuid",
  "createdAt": "2026-03-23T10:00:00"
}
```

#### 2.3.4 停止生成

```
POST /api/conversations/:id/stop

Response 200:
{ "success": true }
```

### 2.4 前端实现示例

```javascript
// 1. 发送消息
async function sendMessage(conversationId, content, modelId) {
  const response = await fetch(`/api/conversations/${conversationId}/messages`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content, modelId })
  });

  const { messageId } = await response.json();

  // 2. 立即建立SSE连接
  streamMessage(messageId);

  return messageId;
}

// 3. SSE流式接收
function streamMessage(messageId) {
  const eventSource = new EventSource(`/api/messages/${messageId}/stream`);

  let accumulatedContent = '';

  eventSource.addEventListener('started', (e) => {
    console.log('Generation started:', e.data);
  });

  eventSource.addEventListener('chunk', (e) => {
    const data = JSON.parse(e.data);
    accumulatedContent += data.content;

    // 实时更新UI（流式显示效果）
    updateMessageUI(messageId, accumulatedContent);
  });

  eventSource.addEventListener('done', (e) => {
    const data = JSON.parse(e.data);
    console.log('Generation completed:', data);
    eventSource.close();
  });

  eventSource.addEventListener('error', (e) => {
    console.error('Stream error:', e);
    eventSource.close();

    // 从数据库读取已生成内容
    fetchMessage(messageId);
  });

  // 4. 处理锁屏/断线
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      // 页面隐藏（锁屏/切换应用）
      // SSE连接可能断开，但后台任务继续运行
      console.log('Page hidden, background task continues');
    } else {
      // 页面恢复
      // 检查消息状态，如果还在生成，重新建立SSE连接
      checkAndReconnect(messageId);
    }
  });
}

// 5. 断线重连
async function checkAndReconnect(messageId) {
  const response = await fetch(`/api/messages/${messageId}`);
  const message = await response.json();

  // 更新UI显示已生成的内容
  updateMessageUI(messageId, message.content);

  // 如果还在生成，重新建立SSE连接
  if (message.status === 'streaming') {
    streamMessage(messageId);
  }
}
```

## 3. 关键特性

### 3.1 流式显示 ✓

**工作原理：**
- 后台任务每收到一个chunk立即发送到watch::channel
- SSE handler实时接收并推送给前端
- 前端EventSource逐字更新UI

**效果：**
- 与ChatGPT完全相同的流式显示效果
- 用户看到AI回复一个字一个字地出现

### 3.2 锁屏继续 ✓

**工作原理：**
- 后台任务在独立的tokio task中运行
- 不依赖任何HTTP连接
- SSE断开不影响任务执行

**效果：**
- 手机锁屏后后台继续生成
- 刷新页面后看到完整内容

### 3.3 断线恢复 ✓

**工作原理：**
- 每个chunk实时写入数据库
- 消息状态持久化（streaming/done/error）
- 重新连接时从数据库读取 + 重新订阅

**效果：**
- 网络波动不影响内容完整性
- 刷新页面后无缝继续

### 3.4 多端同步 ✓

**工作原理：**
- 多个客户端可以同时订阅同一个消息
- 通过watch::channel广播给所有订阅者

**效果：**
- 手机和电脑同时看到生成进度
- 多个浏览器标签页同步

## 4. 实现计划

### Phase 1: 核心基础设施（优先级：高）
- [ ] 扩展 AppState 添加 generation_tasks
- [ ] 实现后台任务管理系统
- [ ] 实现 SSE 订阅系统
- [ ] 实现数据库实时更新逻辑

### Phase 2: 基础流式API（优先级：高）
- [ ] POST /api/conversations/:id/messages（启动任务）
- [ ] GET /api/messages/:id/stream（SSE订阅）
- [ ] GET /api/messages/:id（获取状态）
- [ ] POST /api/conversations/:id/stop（停止生成）

### Phase 3: 高级流式API（优先级：中）
- [ ] POST /api/conversations/:id/resend/stream
- [ ] POST /api/messages/:id/regenerate/stream
- [ ] POST /api/messages/:id/generate-version/stream

### Phase 4: 特殊流式API（优先级：低）
- [ ] POST /api/conversations/:id/compress/stream
- [ ] POST /api/conversations/:id/messages/group/stream

### Phase 5: 测试与优化
- [ ] 单元测试
- [ ] 集成测试
- [ ] 锁屏场景测试
- [ ] 断线重连测试
- [ ] 性能优化

## 5. 技术细节

### 5.1 watch::channel 选择理由

**为什么用 watch 而不是 mpsc？**
- watch 支持多个接收者（多端同步）
- watch 只保留最新值（节省内存）
- watch 适合状态广播场景

### 5.2 数据库更新策略

**chunk级别更新：**
```rust
// 每收到一个chunk
accumulated_content.push_str(&chunk);
db::messages::update_content(conn, &message_id, &accumulated_content);
```

**优化：批量更新**
```rust
// 每100ms或每10个chunk更新一次
if chunks_since_last_update >= 10 || elapsed > 100ms {
    db::messages::update_content(conn, &message_id, &accumulated_content);
}
```

### 5.3 任务清理

**自动清理：**
- 任务完成后自动从 generation_tasks 移除
- 定期清理超时任务（>10分钟）

**手动清理：**
- 用户调用 stop API
- 服务器重启时清理所有任务

## 6. 与ChatGPT对比

| 特性 | ChatGPT | 我们的方案 | 状态 |
|------|---------|-----------|------|
| 流式显示 | ✓ | ✓ | 完全支持 |
| 锁屏继续 | ✓ | ✓ | 完全支持 |
| 断线恢复 | ✓ | ✓ | 完全支持 |
| 多端同步 | ✓ | ✓ | 完全支持 |
| 停止生成 | ✓ | ✓ | 完全支持 |

## 7. 总结

**这个架构完全支持流式显示效果！**

核心原理：
1. 后台任务独立运行，每个chunk实时发送
2. SSE实时接收并推送给前端
3. 前端逐字更新UI（流式效果）
4. 即使SSE断开，后台任务继续，内容持久化
5. 重新连接后无缝继续

**与ChatGPT的效果完全相同：**
- ✅ 流式显示：一个字一个字出现
- ✅ 锁屏继续：后台继续生成
- ✅ 断线恢复：刷新后看到完整内容
- ✅ 多端同步：多设备同时观看

**下一步：**
按照实现计划逐步开发，预计需要：
- 核心基础设施：~200行代码
- 7个流式API：~500行代码
- 测试和优化：实际测试验证

---

生成时间：2026-03-23
作者：Claude Sonnet 4.6
