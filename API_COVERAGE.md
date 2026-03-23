# API Coverage: Desktop vs PWA

## 功能对比表

| 模块 | 桌面端命令 | PWA API 路由 | 状态 |
|------|-----------|-------------|------|
| **Provider 管理** |
| | `add_provider` | `POST /api/providers` | ✅ 已实现 |
| | `list_providers` | `GET /api/providers` | ✅ 已实现 |
| | `update_provider` | `PATCH /api/providers/:id` | ✅ 已实现 |
| | `delete_provider` | `DELETE /api/providers/:id` | ✅ 已实现 |
| | `fetch_models` | `POST /api/providers/:id/fetch-models` | ✅ 已实现 |
| **Model 管理** |
| | `create_manual_model` | `POST /api/providers/:id/models` | ✅ 已实现 |
| | `update_manual_model` | `PATCH /api/models/:id` | ✅ 已实现 |
| | `delete_manual_model` | `DELETE /api/models/:id` | ✅ 已实现 |
| | `update_model_visibility` | `PATCH /api/models/:id/visibility` | ✅ 已实现 |
| | `update_provider_models_visibility` | `PATCH /api/providers/:id/models/visibility` | ✅ 已实现 |
| | - | `GET /api/models` | ✅ 已实现 |
| **Conversation 管理** |
| | `create_conversation` | `POST /api/conversations` | ✅ 已实现 |
| | `list_conversations` | `GET /api/conversations` | ✅ 已实现 |
| | `update_conversation_title` | `PATCH /api/conversations/:id/title` | ✅ 已实现 |
| | `delete_conversation` | `DELETE /api/conversations/:id` | ✅ 已实现 |
| | `pin_conversation` | `PATCH /api/conversations/:id/pin` | ✅ 已实现 |
| | `update_conversation_assistant` | `PATCH /api/conversations/:id/assistant` | ✅ 已实现 |
| | `update_conversation_model` | `PATCH /api/conversations/:id/model` | ✅ 已实现 |
| | `generate_conversation_title` | `POST /api/conversations/:id/generate-title` | ✅ 已实现 |
| | `fork_conversation` | `POST /api/conversations/:id/fork` | ✅ 已实现 |
| **Message 管理** |
| | `get_messages` | `GET /api/conversations/:id/messages` | ✅ 已实现 |
| | `send_message` | `POST /api/conversations/:id/messages` | ⚠️ 部分实现（无流式） |
| | `delete_message` | `DELETE /api/messages/:id` | ✅ 已实现 |
| | `restore_message` | `POST /api/messages/:id/restore` | ✅ 已实现 |
| | `delete_messages_after` | `POST /api/conversations/:id/messages/delete-after` | ✅ 已实现 |
| | `delete_messages_from` | `POST /api/conversations/:id/messages/delete-from` | ✅ 已实现 |
| | `update_message_content` | `PATCH /api/messages/:id/content` | ✅ 已实现 |
| **Paste 管理** |
| | `get_paste_blob_content` | `GET /api/paste/:id` | ✅ 已实现 |
| | `hydrate_paste_content` | `POST /api/paste/hydrate` | ✅ 已实现 |
| | `expand_paste_content` | `POST /api/paste/expand` | ✅ 已实现 |
| **Version 管理** |
| | `switch_version` | `POST /api/messages/:id/switch-version` | ✅ 已实现 |
| | `list_versions` | `GET /api/messages/:id/versions` | ✅ 已实现 |
| | `list_version_messages` | `GET /api/messages/:id/version-messages` | ✅ 已实现 |
| | `get_version_models` | `GET /api/messages/:id/version-models` | ✅ 已实现 |
| **Chat 操作** |
| | `send_message` (streaming) | - | ⚠️ PWA端不支持流式（架构限制） |
| | `resend_message` | `POST /api/conversations/:id/resend` | ⚠️ PWA端不支持（需要流式） |
| | `generate_version` | `POST /api/messages/:id/generate-version` | ⚠️ PWA端不支持（需要流式） |
| | `regenerate_message` | `POST /api/messages/:id/regenerate` | ⚠️ PWA端不支持（需要流式） |
| | `compress_conversation` | `POST /api/conversations/:id/compress` | ⚠️ PWA端不支持（需要流式） |
| | `send_message_group` | `POST /api/conversations/:id/messages/group` | ⚠️ PWA端不支持（需要流式） |
| | `stop_generation` | `POST /api/conversations/:id/stop` | ⚠️ PWA端不支持（需要流式） |
| **Assistant 管理** |
| | `create_assistant` | `POST /api/assistants` | ✅ 已实现 |
| | `list_assistants` | `GET /api/assistants` | ✅ 已实现 |
| | `update_assistant` | `PATCH /api/assistants/:id` | ✅ 已实现 |
| | `delete_assistant` | `DELETE /api/assistants/:id` | ✅ 已实现 |
| **Search 功能** |
| | `search_messages` | `GET /api/search/messages` | ✅ 已实现 |
| | `search_sidebar_results` | `GET /api/search/sidebar` | ✅ 已实现 |
| **Export 功能** |
| | `export_conversation_markdown` | `GET /api/conversations/:id/export/markdown` | ✅ 已实现 |
| | `export_conversation_json` | `GET /api/conversations/:id/export/json` | ✅ 已实现 |
| **Settings 功能** |
| | `get_app_paths` | - | ❌ 不适用（桌面端特有） |
| | `open_path` | - | ❌ 不适用（桌面端特有） |
| | `pick_directory` | - | ❌ 不适用（桌面端特有） |
| | `get_cache_size` | `GET /api/settings/cache-size` | ✅ 已实现 |
| | `clear_cache` | `POST /api/settings/clear-cache` | ✅ 已实现 |
| | `reset_app_data` | `POST /api/settings/reset` | ✅ 已实现 |
| | `local_backup` | - | ❌ 不适用（桌面端特有） |
| | `get_autostart_enabled` | - | ❌ 不适用（桌面端特有） |
| | `set_autostart_enabled` | - | ❌ 不适用（桌面端特有） |
| | `set_proxy_mode` | `POST /api/settings/proxy` | ✅ 已实现 |
| | `get_proxy_mode` | `GET /api/settings/proxy` | ✅ 已实现 |

## 统计

- ✅ 已实现：44 个
- ⚠️ 部分实现/架构限制：8 个（流式响应相关功能需要架构改造）
- ❌ 不适用（桌面端特有）：5 个

**总计**: 58 个桌面端命令中，44 个已在 PWA 端实现，8 个因架构限制暂不支持（需要实现 SSE 或 WebSocket），5 个为桌面端特有功能。

## 优先级建议

### 高优先级（核心功能）
1. **Assistant CRUD** - 创建、更新、删除助手
2. **Conversation 扩展** - pin、更新 assistant/model、生成标题、fork
3. **Message 操作** - 删除、恢复、更新内容
4. **Chat 流式支持** - 实现真正的流式响应
5. **Search 功能** - 消息搜索和侧边栏搜索
6. **Export 功能** - Markdown 和 JSON 导出

### 中优先级（增强功能）
7. **Version 管理** - 切换版本、列出版本
8. **Chat 高级操作** - resend、regenerate、generate_version
9. **Paste 管理** - 获取、展开、水合 paste 内容
10. **Settings** - 代理设置、缓存管理、重置数据

### 低优先级（可选功能）
11. **Chat 高级操作** - compress_conversation、send_message_group
12. **Message 批量操作** - delete_after、delete_from

## 注意事项

1. **流式响应限制**：当前 PWA 端的 `send_message` 只创建用户消息并返回,不支持 AI 流式响应。以下功能因依赖流式响应而暂不支持:
   - `resend_message` - 重新发送消息
   - `generate_version` - 生成新版本回复
   - `regenerate_message` - 重新生成消息
   - `compress_conversation` - 压缩对话
   - `send_message_group` - 多模型并发发送
   - `stop_generation` - 停止生成

   这些功能需要实现 Server-Sent Events (SSE) 或 WebSocket 才能支持。

2. **桌面端特有功能**：某些功能（如文件选择器、自动启动、打开路径）是桌面端特有的，PWA 端不需要实现。

3. **架构差异**：
   - 桌面端使用 Tauri IPC Channel 进行流式传输
   - PWA 端需要使用 HTTP 轮询或 Server-Sent Events (SSE) 实现流式

4. **认证和安全**：PWA 端可能需要添加认证机制，而桌面端不需要。

5. **已实现功能**：除流式相关的 7 个功能外，其他所有适用于 PWA 的功能均已实现，包括:
   - 完整的 Provider 和 Model 管理
   - Conversation 和 Message CRUD 操作
   - Assistant 管理
   - Version 管理
   - Paste 内容处理
   - Search 功能
   - Export 功能（Markdown 和 JSON）
   - Settings 管理（代理、缓存、重置）
