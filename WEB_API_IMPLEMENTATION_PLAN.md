# Web Server API 实现计划

## 实现批次

### 第一批：核心功能（优先级最高）
1. **Assistant CRUD** (3个)
   - POST /api/assistants - create_assistant
   - PATCH /api/assistants/:id - update_assistant
   - DELETE /api/assistants/:id - delete_assistant

2. **Conversation 扩展** (5个)
   - PATCH /api/conversations/:id/pin - pin_conversation
   - PATCH /api/conversations/:id/assistant - update_conversation_assistant
   - PATCH /api/conversations/:id/model - update_conversation_model
   - POST /api/conversations/:id/generate-title - generate_conversation_title
   - POST /api/conversations/:id/fork - fork_conversation

3. **Message 操作** (5个)
   - DELETE /api/messages/:id - delete_message
   - POST /api/messages/:id/restore - restore_message
   - POST /api/conversations/:id/messages/delete-after - delete_messages_after
   - POST /api/conversations/:id/messages/delete-from - delete_messages_from
   - PATCH /api/messages/:id/content - update_message_content

### 第二批：搜索和导出（优先级高）
4. **Search 功能** (2个)
   - GET /api/search/messages?query= - search_messages
   - GET /api/search/sidebar?query= - search_sidebar_results

5. **Export 功能** (2个)
   - GET /api/conversations/:id/export/markdown - export_conversation_markdown
   - GET /api/conversations/:id/export/json - export_conversation_json

### 第三批：Version 和 Paste 管理（优先级中）
6. **Version 管理** (4个)
   - POST /api/messages/:id/switch-version - switch_version
   - GET /api/messages/:id/versions - list_versions
   - GET /api/messages/:id/version-messages - list_version_messages
   - GET /api/messages/:id/version-models - get_version_models

7. **Paste 管理** (3个)
   - GET /api/paste/:id - get_paste_blob_content
   - POST /api/paste/hydrate - hydrate_paste_content
   - POST /api/paste/expand - expand_paste_content

### 第四批：Settings（优先级中）
8. **Settings 功能** (3个)
   - GET /api/settings/proxy - get_proxy_mode
   - POST /api/settings/proxy - set_proxy_mode
   - POST /api/settings/reset - reset_app_data

### 第五批：Chat 高级操作（优先级低，需要架构调整）
9. **Chat 流式支持** - 需要重构 send_message 实现后台任务
10. **Chat 高级操作** (6个) - 需要流式支持后才能实现
   - POST /api/conversations/:id/resend
   - POST /api/messages/:id/generate-version
   - POST /api/messages/:id/regenerate
   - POST /api/conversations/:id/compress
   - POST /api/conversations/:id/messages/group
   - POST /api/conversations/:id/stop

## 实现状态
- ✅ 已完成：Provider CRUD (10个)
- ✅ 已完成：Model 管理 (5个)
- ✅ 已完成：Conversation 基础 (4个)
- ✅ 已完成：Message 基础 (2个)
- ✅ 已完成：Assistant 列表 (1个)
- 🔄 进行中：第一批核心功能
- ⏳ 待实现：第二批到第五批

## 总计
- 已实现：22个
- 待实现：36个（不含桌面端特有的5个）
