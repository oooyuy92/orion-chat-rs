#!/bin/bash

echo "=== 桌面端所有 Tauri Commands ==="
echo ""

echo "## Provider 管理 (10个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/provider.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Conversation 管理 (5个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/conversation.rs | grep "pub async fn" | head -5 | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Message 管理 (18个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/conversation.rs | grep "pub async fn" | tail -n +6 | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Chat 操作 (7个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/chat.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Assistant 管理 (4个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/assistant.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Search 功能 (2个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/search.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Export 功能 (2个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/export.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "## Settings 功能 (10个)"
grep -A1 "#\[tauri::command\]" src-tauri/src/commands/settings.rs | grep "pub async fn" | sed 's/.*fn \([^(]*\).*/\1/'

echo ""
echo "=== 总计 ==="
grep -r "#\[tauri::command\]" src-tauri/src/commands/ | wc -l | xargs echo "总命令数:"
