# yoagent Phase 1 — 前端 Agent UI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Orion Chat 前端实现 Agent 模式切换按钮、工具执行时间线组件和授权弹窗，使用户可以看到 Agent 的工具调用过程并进行授权操作。

**Architecture:** 在现有 InputArea.svelte 的 model-row 中添加 Agent 切换按钮（hover 展开），新建 ToolTimeline 组件渲染工具调用步骤，新建 ToolAuthDialog 组件处理授权弹窗。前端通过 Tauri Channel 接收新增的 ChatEvent 变体（ToolCallStart/Update/End/ToolAuthRequest）。

**Tech Stack:** Svelte 5, shadcn-svelte, Lucide Svelte (`@lucide/svelte`), Tauri v2 IPC, TypeScript

**前置依赖:** Phase 0 后端核心集成已完成。

---

## 文件结构

### 新建文件
- `src/lib/components/chat/AgentToggle.svelte` — Agent 模式切换按钮（hover 展开，线条 bot 图标）
- `src/lib/components/chat/ToolTimeline.svelte` — 工具调用步骤时间线组件
- `src/lib/components/chat/ToolTimelineItem.svelte` — 单个工具调用步骤项（展开/折叠详情）
- `src/lib/components/chat/ToolAuthDialog.svelte` — 工具授权确认弹窗
- `src/lib/stores/agent.ts` — Agent 状态 store（agent_mode、pending tool calls）
- `src/lib/api/agent.ts` — Agent Tauri command 调用封装

### 修改文件
- `src/lib/components/chat/InputArea.svelte` — model-row 中添加 AgentToggle
- `src/lib/components/chat/MessageList.svelte`（或渲染消息的组件）— 识别并渲染 tool_call/tool_result 消息
- `src/lib/api/invoke.ts`（或 Tauri IPC 封装）— 添加 agent_chat / agent_stop 调用
- `src/lib/types/` — 扩展 Message 和 ChatEvent TypeScript 类型

---

## Chunk 0: TypeScript 类型和 API 封装

### Task 0: 扩展 TypeScript 类型定义

**Files:**
- Modify: `src/lib/types/` 中的消息类型文件

- [ ] **Step 1: 找到现有的 Message 和 ChatEvent TS 类型定义**

```bash
grep -rn "interface Message\|type Message\|ChatEvent\|messageId\|type.*=.*Delta" src/lib/types/ src/lib/ --include="*.ts" | head -20
```

- [ ] **Step 2: 扩展 Message 类型**

在已有 `Message` interface 中添加：

```typescript
export type MessageType = 'text' | 'toolCall' | 'toolResult';

export interface Message {
  // ... 现有字段 ...
  messageType: MessageType;
  toolCallId?: string;
  toolName?: string;
  toolInput?: string;
  toolError: boolean;
}
```

- [ ] **Step 3: 扩展 ChatEvent 类型**

```typescript
export type ChatEvent =
  | { type: 'started'; messageId: string }
  | { type: 'delta'; messageId: string; content: string }
  | { type: 'reasoning'; messageId: string; content: string }
  | { type: 'usage'; messageId: string; promptTokens: number; completionTokens: number }
  | { type: 'finished'; messageId: string }
  | { type: 'error'; messageId: string; message: string }
  // Agent 新增事件
  | { type: 'toolCallStart'; messageId: string; toolCallId: string; toolName: string; args: string }
  | { type: 'toolCallUpdate'; messageId: string; toolCallId: string; partialResult: string }
  | { type: 'toolCallEnd'; messageId: string; toolCallId: string; result: string; isError: boolean }
  | { type: 'toolAuthRequest'; toolCallId: string; toolName: string; args: string };
```

- [ ] **Step 4: 添加 Agent 相关类型**

```typescript
export type PermissionLevel = 'auto' | 'ask' | 'deny';
export type AuthAction = 'allow' | 'allowSession' | 'deny';

export interface ToolPermissions {
  [toolName: string]: PermissionLevel;
}

export interface ToolCallState {
  toolCallId: string;
  toolName: string;
  args: string;
  status: 'running' | 'completed' | 'error';
  result?: string;
  messageId: string;
  startTime: number;
  endTime?: number;
}
```

- [ ] **Step 5: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 6: 提交**

```bash
git add src/lib/types/
git commit -m "feat: extend TypeScript types for Agent ChatEvent and ToolCallState"
```

### Task 1: Agent API 封装

**Files:**
- Create: `src/lib/api/agent.ts`

- [ ] **Step 1: 查看现有的 Tauri invoke 封装方式**

```bash
grep -rn "invoke\|@tauri-apps" src/lib/api/ src/lib/ --include="*.ts" | head -20
```

- [ ] **Step 2: 创建 agent.ts**

```typescript
// src/lib/api/agent.ts
import { invoke, Channel } from '@tauri-apps/api/core';
import type { Message, ChatEvent, AuthAction, ToolPermissions } from '$lib/types';

export async function agentChat(
  conversationId: string,
  message: string,
  modelId: string,
  onEvent: (event: ChatEvent) => void,
): Promise<Message> {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = onEvent;

  return invoke<Message>('agent_chat', {
    conversationId,
    message,
    modelId,
    channel,
  });
}

export async function agentStop(conversationId: string): Promise<void> {
  return invoke('agent_stop', { conversationId });
}

export async function agentAuthorizeTool(
  toolCallId: string,
  action: AuthAction,
): Promise<void> {
  return invoke('agent_authorize_tool', { toolCallId, action });
}

export async function getToolPermissions(): Promise<ToolPermissions> {
  return invoke<ToolPermissions>('get_tool_permissions');
}

export async function setToolPermissions(perms: ToolPermissions): Promise<void> {
  return invoke('set_tool_permissions', { perms });
}
```

- [ ] **Step 3: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 4: 提交**

```bash
git add src/lib/api/agent.ts
git commit -m "feat: agent API layer - agentChat, agentStop, agentAuthorizeTool"
```

---

## Chunk 1: Agent 状态管理

### Task 2: Agent Store

**Files:**
- Create: `src/lib/stores/agent.ts`

- [ ] **Step 1: 查看现有 store 模式**

```bash
ls src/lib/stores/ 2>/dev/null || ls src/lib/state/ 2>/dev/null
grep -rn "writable\|readable\|store\|rune\|\$state" src/lib/stores/ --include="*.ts" --include="*.svelte" | head -10
```

确定项目使用 Svelte 5 runes（`$state`）还是 Svelte 4 stores（`writable`）。

- [ ] **Step 2: 创建 agent store**

按照项目已有的 store 模式创建。以下示例使用 Svelte 5 runes 风格（若项目用 writable 则调整）：

```typescript
// src/lib/stores/agent.ts
import { writable } from 'svelte/store';
import type { ToolCallState } from '$lib/types';

// 当前会话是否为 Agent 模式
export const agentMode = writable<boolean>(true);

// 当前正在执行的工具调用列表
export const activeToolCalls = writable<ToolCallState[]>([]);

// 待授权的工具调用
export const pendingAuth = writable<{
  toolCallId: string;
  toolName: string;
  args: string;
} | null>(null);

// 辅助函数
export function addToolCall(call: ToolCallState) {
  activeToolCalls.update(calls => [...calls, call]);
}

export function updateToolCall(toolCallId: string, partial: Partial<ToolCallState>) {
  activeToolCalls.update(calls =>
    calls.map(c => c.toolCallId === toolCallId ? { ...c, ...partial } : c),
  );
}

export function completeToolCall(toolCallId: string, result: string, isError: boolean) {
  activeToolCalls.update(calls =>
    calls.map(c =>
      c.toolCallId === toolCallId
        ? { ...c, status: isError ? 'error' : 'completed', result, endTime: Date.now() }
        : c,
    ),
  );
}

export function clearToolCalls() {
  activeToolCalls.set([]);
}
```

- [ ] **Step 3: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 4: 提交**

```bash
git add src/lib/stores/agent.ts
git commit -m "feat: agent store - agentMode, activeToolCalls, pendingAuth state management"
```

---

## Chunk 2: Agent 切换按钮

### Task 3: AgentToggle 组件

**Files:**
- Create: `src/lib/components/chat/AgentToggle.svelte`
- Modify: `src/lib/components/chat/InputArea.svelte`

- [ ] **Step 1: 查看 InputArea.svelte 的 model-row 结构**

```bash
sed -n '155,175p' src/lib/components/chat/InputArea.svelte
```

- [ ] **Step 2: 创建 AgentToggle.svelte**

```svelte
<!-- src/lib/components/chat/AgentToggle.svelte -->
<script lang="ts">
  import BotIcon from '@lucide/svelte/icons/bot';
  import { agentMode } from '$lib/stores/agent';

  let { disabled = false }: { disabled?: boolean } = $props();

  function toggle() {
    if (!disabled) {
      agentMode.update(v => !v);
    }
  }
</script>

<button
  class="agent-toggle"
  class:active={$agentMode}
  title={$agentMode ? 'Agent 模式（点击关闭）' : 'Agent 模式（点击开启）'}
  onclick={toggle}
  {disabled}
>
  <BotIcon class="h-3.5 w-3.5" />
  <span class="agent-label">
    Agent
    <span class="agent-badge">{$agentMode ? 'ON' : 'OFF'}</span>
  </span>
</button>

<style>
  .agent-toggle {
    display: flex;
    align-items: center;
    gap: 0;
    border-radius: 7px;
    padding: 4px 7px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid hsl(var(--border));
    background: hsl(var(--muted));
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
    overflow: hidden;
    max-width: 27px;
    transition:
      max-width 0.22s ease,
      gap 0.18s ease,
      padding 0.18s ease,
      background 0.15s ease,
      color 0.15s ease;
  }

  .agent-toggle:hover {
    max-width: 120px;
    gap: 5px;
    padding: 4px 10px;
  }

  .agent-toggle.active {
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    border-color: hsl(var(--primary));
  }

  .agent-label {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    width: 0;
    overflow: hidden;
    transition:
      opacity 0.15s ease 0.05s,
      width 0.22s ease;
  }

  .agent-toggle:hover .agent-label {
    opacity: 1;
    width: auto;
  }

  .agent-badge {
    border-radius: 4px;
    padding: 0 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .agent-toggle.active .agent-badge {
    background: hsl(var(--primary-foreground) / 0.15);
  }

  .agent-toggle:not(.active) .agent-badge {
    background: hsl(var(--border));
  }

  .agent-toggle:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

- [ ] **Step 3: 在 InputArea.svelte 的 model-row 中添加 AgentToggle**

在 `model-row` div 内部，ComboSelector 之后添加：

```svelte
<script>
  // 在已有 import 后添加
  import AgentToggle from './AgentToggle.svelte';
</script>

<!-- 在 model-row div 内，ComboSelector 之后 -->
<div class="model-row">
  <ModelSelector ... />
  <ModelParamsPopover ... />
  <ComboSelector ... />
  <div style="flex:1"></div>
  <AgentToggle {disabled} />
</div>
```

- [ ] **Step 4: 编译并在浏览器中验证**

```bash
pnpm dev 2>&1 &
# 打开浏览器访问应用，检查输入框上方是否出现 bot 图标
# hover 验证展开动画
# 点击验证 ON/OFF 切换
```

- [ ] **Step 5: 提交**

```bash
git add src/lib/components/chat/AgentToggle.svelte src/lib/components/chat/InputArea.svelte
git commit -m "feat: AgentToggle button in model-row with hover expand animation"
```

---

## Chunk 3: 工具时间线组件

### Task 4: ToolTimelineItem 组件

**Files:**
- Create: `src/lib/components/chat/ToolTimelineItem.svelte`

- [ ] **Step 1: 创建 ToolTimelineItem**

```svelte
<!-- src/lib/components/chat/ToolTimelineItem.svelte -->
<script lang="ts">
  import CheckIcon from '@lucide/svelte/icons/check';
  import LoaderIcon from '@lucide/svelte/icons/loader';
  import XIcon from '@lucide/svelte/icons/x';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import type { ToolCallState } from '$lib/types';

  let { call }: { call: ToolCallState } = $props();

  let expanded = $state(false);

  const elapsed = $derived(
    call.endTime
      ? `${((call.endTime - call.startTime) / 1000).toFixed(1)}s`
      : '...',
  );

  function formatArgs(args: string): string {
    try {
      const parsed = JSON.parse(args);
      // 取第一个值作为摘要
      const values = Object.values(parsed);
      return values.length > 0 ? String(values[0]).slice(0, 60) : '';
    } catch {
      return args.slice(0, 60);
    }
  }
</script>

<div class="timeline-item" class:expanded>
  <button class="timeline-row" onclick={() => (expanded = !expanded)}>
    <span class="status-icon">
      {#if call.status === 'completed'}
        <CheckIcon class="h-3.5 w-3.5 text-green-600" />
      {:else if call.status === 'error'}
        <XIcon class="h-3.5 w-3.5 text-red-500" />
      {:else}
        <LoaderIcon class="h-3.5 w-3.5 text-yellow-500 animate-spin" />
      {/if}
    </span>
    <span class="tool-name">{call.toolName}</span>
    <span class="tool-summary">{formatArgs(call.args)}</span>
    <span class="elapsed">{elapsed}</span>
    <ChevronRightIcon
      class="h-3 w-3 chevron"
      style:transform={expanded ? 'rotate(90deg)' : 'rotate(0deg)'}
    />
  </button>

  {#if expanded && call.result}
    <pre class="tool-output">{call.result}</pre>
  {/if}
</div>

<style>
  .timeline-item {
    font-size: 12px;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    background: none;
    border: none;
    padding: 3px 0;
    cursor: pointer;
    text-align: left;
    color: hsl(var(--muted-foreground));
  }

  .timeline-row:hover {
    color: hsl(var(--foreground));
  }

  .tool-name {
    font-weight: 500;
    color: hsl(var(--foreground));
  }

  .tool-summary {
    color: hsl(var(--muted-foreground));
    font-family: monospace;
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 200px;
  }

  .elapsed {
    margin-left: auto;
    font-size: 10px;
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
  }

  .chevron {
    transition: transform 0.15s ease;
    flex-shrink: 0;
  }

  .tool-output {
    margin: 4px 0 4px 22px;
    padding: 8px;
    background: hsl(var(--muted));
    border-radius: 6px;
    font-size: 11px;
    font-family: monospace;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
    color: hsl(var(--foreground));
  }
</style>
```

### Task 5: ToolTimeline 组件

**Files:**
- Create: `src/lib/components/chat/ToolTimeline.svelte`

- [ ] **Step 1: 创建 ToolTimeline**

```svelte
<!-- src/lib/components/chat/ToolTimeline.svelte -->
<script lang="ts">
  import ToolTimelineItem from './ToolTimelineItem.svelte';
  import type { ToolCallState } from '$lib/types';

  let { calls }: { calls: ToolCallState[] } = $props();

  const hasRunning = $derived(calls.some(c => c.status === 'running'));
</script>

{#if calls.length > 0}
  <div class="tool-timeline">
    <div class="timeline-label">
      {hasRunning ? 'Agent 执行中' : 'Agent 执行完成'}
    </div>
    {#each calls as call (call.toolCallId)}
      <ToolTimelineItem {call} />
    {/each}
  </div>
{/if}

<style>
  .tool-timeline {
    border-left: 2px solid hsl(var(--border));
    padding-left: 10px;
    margin: 4px 0 4px 3px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .timeline-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: hsl(var(--muted-foreground));
    margin-bottom: 2px;
  }
</style>
```

- [ ] **Step 2: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 3: 提交**

```bash
git add src/lib/components/chat/ToolTimeline.svelte src/lib/components/chat/ToolTimelineItem.svelte
git commit -m "feat: ToolTimeline and ToolTimelineItem components for agent tool execution display"
```

---

## Chunk 4: 授权弹窗

### Task 6: ToolAuthDialog 组件

**Files:**
- Create: `src/lib/components/chat/ToolAuthDialog.svelte`

- [ ] **Step 1: 查看项目中已有的 Dialog 使用方式**

```bash
grep -rn "Dialog\|AlertDialog\|dialog" src/lib/components/ --include="*.svelte" | head -15
```

- [ ] **Step 2: 创建 ToolAuthDialog**

```svelte
<!-- src/lib/components/chat/ToolAuthDialog.svelte -->
<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { pendingAuth } from '$lib/stores/agent';
  import { agentAuthorizeTool } from '$lib/api/agent';
  import type { AuthAction } from '$lib/types';

  const open = $derived($pendingAuth !== null);

  async function respond(action: AuthAction) {
    if ($pendingAuth) {
      await agentAuthorizeTool($pendingAuth.toolCallId, action);
      pendingAuth.set(null);
    }
  }

  function formatArgs(args: string): string {
    try {
      return JSON.stringify(JSON.parse(args), null, 2);
    } catch {
      return args;
    }
  }
</script>

<Dialog.Root {open} onOpenChange={(v) => { if (!v) respond('deny'); }}>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title>工具授权请求</Dialog.Title>
      <Dialog.Description>
        Agent 请求执行以下工具，是否允许？
      </Dialog.Description>
    </Dialog.Header>

    {#if $pendingAuth}
      <div class="space-y-3">
        <div class="flex items-center gap-2">
          <span class="font-mono text-sm font-semibold">{$pendingAuth.toolName}</span>
        </div>
        <pre class="rounded-md bg-muted p-3 text-xs font-mono max-h-48 overflow-auto">{formatArgs($pendingAuth.args)}</pre>
      </div>
    {/if}

    <Dialog.Footer class="flex gap-2 sm:justify-between">
      <Button variant="destructive" size="sm" onclick={() => respond('deny')}>
        拒绝
      </Button>
      <div class="flex gap-2">
        <Button variant="outline" size="sm" onclick={() => respond('allowSession')}>
          允许并记住
        </Button>
        <Button size="sm" onclick={() => respond('allow')}>
          允许
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
```

- [ ] **Step 3: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

按编译错误调整 shadcn-svelte Dialog 的导入路径（可能是 `$lib/components/ui/dialog/index.ts` 或其他路径）。

- [ ] **Step 4: 提交**

```bash
git add src/lib/components/chat/ToolAuthDialog.svelte
git commit -m "feat: ToolAuthDialog - permission prompt with allow/allowSession/deny actions"
```

---

## Chunk 5: 集成到消息流

### Task 7: 将 Agent 事件集成到消息渲染和发送流程

**Files:**
- Modify: 消息渲染组件（MessageList / MessageItem 等）
- Modify: 消息发送逻辑（InputArea 或 chat store）

- [ ] **Step 1: 找到消息发送逻辑和消息渲染组件**

```bash
grep -rn "send_message\|sendMessage\|invoke.*send" src/lib/ --include="*.ts" --include="*.svelte" | head -20
grep -rn "messages.*each\|#each.*message" src/lib/components/chat/ --include="*.svelte" | head -10
```

- [ ] **Step 2: 修改消息发送逻辑以支持 Agent 模式**

找到调用 `invoke('send_message', ...)` 的位置，添加 Agent 模式分支：

```typescript
import { get } from 'svelte/store';
import { agentMode, addToolCall, updateToolCall, completeToolCall, clearToolCalls, pendingAuth } from '$lib/stores/agent';
import { agentChat } from '$lib/api/agent';

// 在发送消息的函数中
async function handleSend(content: string) {
  if (get(agentMode)) {
    clearToolCalls();
    await agentChat(conversationId, content, modelId, (event) => {
      switch (event.type) {
        case 'toolCallStart':
          addToolCall({
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            args: event.args,
            status: 'running',
            messageId: event.messageId,
            startTime: Date.now(),
          });
          break;
        case 'toolCallUpdate':
          updateToolCall(event.toolCallId, { result: event.partialResult });
          break;
        case 'toolCallEnd':
          completeToolCall(event.toolCallId, event.result, event.isError);
          break;
        case 'toolAuthRequest':
          pendingAuth.set({
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            args: event.args,
          });
          break;
        // delta, started, finished 等事件复用现有处理逻辑
        default:
          handleExistingEvent(event);
      }
    });
  } else {
    // 现有普通聊天发送逻辑
    await sendMessage(conversationId, content, modelId, ...);
  }
}
```

- [ ] **Step 3: 在消息渲染中集成 ToolTimeline**

找到渲染 assistant 消息的组件，在助手消息前后插入时间线：

```svelte
<script>
  import ToolTimeline from './ToolTimeline.svelte';
  import { activeToolCalls } from '$lib/stores/agent';
</script>

<!-- 在消息列表渲染区域 -->
{#each messages as message}
  {#if message.role === 'assistant'}
    <!-- 在助手消息前展示该消息关联的工具调用时间线 -->
    <ToolTimeline calls={$activeToolCalls} />
  {/if}
  <MessageBubble {message} />
{/each}
```

**注意**：实际集成需按项目已有的消息渲染结构调整。工具调用时间线应该与对应的 assistant 回复消息关联。

- [ ] **Step 4: 在根布局或 Chat 页面中挂载 ToolAuthDialog**

找到 Chat 页面组件（包含 MessageList 和 InputArea 的父组件），添加：

```svelte
<script>
  import ToolAuthDialog from './ToolAuthDialog.svelte';
</script>

<!-- 在组件底部 -->
<ToolAuthDialog />
```

- [ ] **Step 5: 编译验证**

```bash
pnpm check 2>&1 | tail -10
```

- [ ] **Step 6: 提交**

```bash
git add -u
git commit -m "feat: integrate Agent events into message flow - timeline, auth dialog, mode switch"
```

---

## Chunk 6: 端到端验收

### Task 8: 端到端 UI 验收测试

- [ ] **Step 1: 启动应用**

```bash
pnpm tauri dev
```

- [ ] **Step 2: 验证 Agent 切换按钮**

1. 在输入框上方 model-row 右侧看到 bot 线条图标
2. Hover 后平滑展开显示 "Agent ON"
3. 点击后变为 "Agent OFF"（灰色），再点击恢复 "Agent ON"（primary 色）

- [ ] **Step 3: 验证工具调用时间线**

1. Agent ON 状态下发送 "请列出当前目录的文件"
2. 消息区域出现 `border-left` 时间线，显示 `list_files` 工具
3. 绿色 ✓ 图标表示完成，显示耗时
4. 点击时间线项展开查看完整输出

- [ ] **Step 4: 验证授权弹窗**

1. 发送 "运行 echo hello world"
2. 弹出 Dialog，显示工具名 `bash` 和参数
3. 点击 "允许" 后工具执行并在时间线中显示结果
4. 再次发送 bash 请求，验证 "允许并记住" 功能（同会话内不再弹窗）

- [ ] **Step 5: 验证普通聊天模式**

1. 点击 Agent OFF 关闭 Agent 模式
2. 发送普通消息
3. 不出现工具调用，走现有 send_message 流程

- [ ] **Step 6: 最终提交**

```bash
git add -A
git commit -m "feat: Phase 1 complete - Agent frontend UI (toggle, timeline, auth dialog)"
```

---

## 验收标准

Phase 1 完成的标志：
- [ ] Agent 切换按钮在 model-row 右侧正确渲染，hover 展开动画流畅
- [ ] ON 状态使用 `--primary` 色，OFF 状态使用 `--muted` 色
- [ ] Agent 模式下发送消息走 `agent_chat` 通道
- [ ] 工具调用以 `border-left` 时间线形式展示
- [ ] 时间线项可点击展开查看详情
- [ ] `ask` 权限工具弹出授权 Dialog
- [ ] Dialog 的三个按钮（允许/允许并记住/拒绝）功能正常
- [ ] Agent OFF 模式下消息走普通 `send_message` 通道
- [ ] `pnpm check` 无 TypeScript 报错
