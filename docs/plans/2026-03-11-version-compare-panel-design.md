# UI/UX 设计方案：AI 消息多版本对比面板

**设计时间**：2026-03-11
**目标平台**：Tauri v2 桌面应用 (macOS / Windows / Linux)

---

## 1. 设计目标

### 1.1 用户目标
- 在同一视图中并排查看同一条 AI 消息的所有版本回复，快速比较差异
- 不中断当前聊天上下文，就地展开/收起对比面板
- 清楚知道当前选中的是哪个版本，并可在对比面板中直接切换

### 1.2 业务目标
- 提升多版本生成功能的使用率，让用户直观感受不同版本质量差异
- 复用现有 `CompareView` 的卡片设计语言，保持 UI 一致性
- 不引入额外后端接口，使用已有 `api.listVersions()` 和 `api.getMessages()` 获取数据

---

## 2. 页面结构设计

### 2.1 修改区域定位

修改仅发生在 `MessageBubble.svelte` 的 assistant 分支（第 405-515 行），具体是在版本按钮组（第 413-426 行）与消息内容区（第 428 行起）之间插入可折叠的对比面板。

### 2.2 展开前布局（现状）

```
+-----------------------------------------------------+
|  [v1] [v2] [v3]                     (版本按钮组)      |
+-----------------------------------------------------+
|                                                      |
|  AI 消息内容 (当前版本的 markdown 渲染)                 |
|                                                      |
+-----------------------------------------------------+
|  [Regenerate] [NewVersion] [Copy] [Delete]           |
+-----------------------------------------------------+
```

### 2.3 展开后布局

```
+-----------------------------------------------------+
|  [v1] [v2] [v3]  [<->]              (版本按钮组 + 对比按钮) |
+-----------------------------------------------------+
|  横向滚动对比面板 (slide-down 展开)                      |
|  +-------------+ +-------------+ +-------------+    |
|  | v1           | | v2 (当前)   | | v3           |    |
|  | Model: GPT-4 | | Model: ...  | | Model: ...  |    |
|  |-------------| |*************| |-------------|    |
|  |              | |  (高亮边框)  | |              |    |
|  | markdown     | | markdown    | | markdown    |    |
|  | 内容渲染      | | 内容渲染     | | 内容渲染     |    |
|  |              | |             | |              |    |
|  +-------------+ +-------------+ +-------------+    |
|                   <-- 横向可滚动 -->                    |
+-----------------------------------------------------+
|  (消息内容区域被隐藏，面板即是内容展示区)                   |
+-----------------------------------------------------+
|  [Regenerate] [NewVersion] [Copy] [Delete]           |
+-----------------------------------------------------+
```

### 2.4 区块说明

| 区块 | 用途 | 变更 |
|------|------|------|
| 版本按钮组 | 切换单版本显示 | 末尾追加"展开对比"图标按钮 |
| 对比面板 | 并排横向滚动展示所有版本 | **新增**，展开时替代单版本内容区 |
| 消息内容区 | 单版本 markdown 渲染 | 对比面板展开时隐藏 |
| 动作按钮组 | 操作按钮 | 不变 |

---

## 3. 组件拆分

### 3.1 组件树结构（修改后）

```
MessageBubble (assistant 分支)
├── VersionButtonGroup (现有，内联)
│   ├── VersionButton x N (现有，内联)
│   └── CompareToggleButton          ← 新增
├── VersionComparePanel              ← 新增组件
│   └── VersionCard x N              ← 新增组件
│       ├── CardHeader (版本号 + 模型名)
│       └── CardBody (markdown 渲染内容)
├── MessageContent (现有，条件隐藏)
├── ReasoningSection (现有)
└── ActionButtons (现有)
```

### 3.2 组件详细定义

#### 组件 A: `CompareToggleButton`（内联在 MessageBubble 中）

**职责**：切换对比面板的展开/收起状态。

不需要独立组件文件，作为内联按钮直接写在版本按钮组的 `{#each}` 循环后面。

**视觉规格**：
- 与现有版本按钮（`v1`, `v2`）同行排列
- 使用 `Columns2` 图标（lucide-svelte），尺寸 14px
- 未展开状态：`text-muted-foreground`，hover 时 `hover:bg-muted hover:text-foreground`
- 展开状态：`bg-foreground text-background`（与选中的版本按钮同风格高亮）
- 按钮尺寸：`p-1 rounded`，与现有动作按钮一致

**交互**：
- 点击切换 `isCompareOpen` 布尔值
- title 属性：展开时显示"收起对比" / 收起时显示"展开对比"（需 i18n）

**示例代码结构**：

```svelte
<button
  class="rounded p-1 cursor-pointer {isCompareOpen
    ? 'bg-foreground text-background'
    : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
  title={isCompareOpen ? i18n.t.collapseCompare : i18n.t.expandCompare}
  onclick={() => toggleCompare()}
>
  <Columns2Icon size={14} />
</button>
```

---

#### 组件 B: `VersionComparePanel.svelte`（新建独立组件）

**职责**：横向滚动容器，加载并展示所有版本消息卡片。

**文件路径**：`src/lib/components/chat/VersionComparePanel.svelte`

**Props 接口**：

```typescript
interface VersionComparePanelProps {
  // 版本组 ID，用于调用 api.listVersions()
  versionGroupId: string;
  // 当前选中的版本号，用于高亮卡片边框
  currentVersionNumber: number;
  // 版本总数，用于初始化占位
  totalVersions: number;
  // 切换版本回调
  onSwitchVersion: (versionNumber: number) => void;
}
```

**内部状态**：

```typescript
let versions: VersionInfo[] = $state([]);           // 版本列表（来自 api.listVersions）
let versionMessages: Map<string, Message> = $state(new Map()); // messageId -> Message 内容
let isLoading: boolean = $state(true);               // 首次加载状态
let loadError: string = $state('');                  // 加载错误信息
```

**数据加载流程**：

1. 组件挂载时调用 `api.listVersions(versionGroupId)` 获取 `VersionInfo[]`
2. 对每个版本，使用 `api.getMessages()` 获取对应的 Message（或者直接从父组件的 messages 数组中查找同组版本）
3. 注意：当前 `messages` 列表中只有当前选中版本的 Message，其他版本需要通过 `listVersions` 返回的 `VersionInfo.id` 作为 messageId 获取内容

**备选方案（推荐）**：在 `MessageBubble` 的父组件 `MessageList` 中，可以不为此功能新增 API 调用。更轻量的方案是利用已有的 `api.listVersions()` 拿到每个版本的 `id`（即 messageId），然后逐个调用一个新的 `api.getMessage(id)` 获取单条消息。但观察现有 API 中没有 `getMessage(id)` 单条接口。因此建议：

- **方案 1**（推荐）：新增 Rust 端 `get_version_messages(versionGroupId)` 命令，一次返回该版本组所有 Message 完整内容
- **方案 2**：复用 `api.listVersions()` 获取 ID 列表后，通过 `api.getMessages()` 分别获取（需要改造后端接口，不推荐）
- **方案 3**（最小化后端改动）：新增前端 `api.getMessageById(id): Promise<Message>` 调用，后端新增对应 `get_message_by_id` 命令

**样式要点**：

```svelte
<div class="flex gap-3 overflow-x-auto py-3 px-1"
     style="scrollbar-width: thin;">
  {#each versions as ver (ver.id)}
    <VersionCard
      versionInfo={ver}
      message={versionMessages.get(ver.id)}
      isActive={ver.versionNumber === currentVersionNumber}
      onclick={() => onSwitchVersion(ver.versionNumber)}
    />
  {/each}
</div>
```

**Loading 状态 UI**：
- 显示与 `totalVersions` 相同数量的骨架卡片
- 骨架卡片：固定宽度 `360px`，高度 `200px`，`animate-pulse` 效果，`bg-muted rounded-xl`

**Error 状态 UI**：
- 显示一行错误提示文字 + 重试按钮

---

#### 组件 C: `VersionCard.svelte`（新建独立组件）

**职责**：单个版本的卡片展示，包含 header（版本号 + 模型名）和 body（markdown 内容）。

**文件路径**：`src/lib/components/chat/VersionCard.svelte`

**Props 接口**：

```typescript
interface VersionCardProps {
  versionNumber: number;
  modelId: string | null;
  modelName: string;         // 模型显示名称（由父组件解析后传入）
  content: string;           // 消息原始内容
  reasoning: string | null;  // 思考过程
  isActive: boolean;         // 是否为当前选中版本
  isLoading?: boolean;       // 该卡片是否正在加载
  onclick?: () => void;      // 点击卡片切换版本
}
```

**视觉规格**：

- **卡片尺寸**：`width: 380px`，`min-height: 200px`，`max-height: 60vh`
- **flex-shrink**: `0`（不压缩，保证横向滚动）
- **边框**：默认 `border border-border`；活跃版本 `border-2 border-primary` 或 `ring-2 ring-primary`
- **背景**：`bg-card`
- **圆角**：`rounded-xl`

**Header 区域**：
- 左侧：版本号标签 `v{n}`，使用 `text-xs font-semibold`
- 右侧：模型名称，使用 `text-xs text-muted-foreground truncate`
- 底部分割线：`border-b border-border`
- 内边距：`px-4 py-2`

**Body 区域**：
- 内边距：`p-3`
- 纵向可滚动：`overflow-y-auto`，配合 `max-height` 约束
- 使用 `renderMarkdown()` 渲染内容
- 复用 `.message-markdown` 样式类
- `flex-1` 填充剩余空间

**交互**：
- 整张卡片可点击，点击后触发 `onSwitchVersion(versionNumber)`
- hover 效果：`hover:shadow-md` 提升层次感
- 点击后对比面板不关闭，仅更新高亮卡片（让用户可以继续对比）
- 光标：`cursor-pointer`

**示例代码结构**：

```svelte
<script lang="ts">
  import { renderMarkdown } from '$lib/utils/markdown';

  let {
    versionNumber,
    modelName,
    content,
    reasoning,
    isActive,
    isLoading = false,
    onclick,
  }: VersionCardProps = $props();

  const renderedContent = $derived(renderMarkdown(content));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="flex-shrink-0 rounded-xl flex flex-col cursor-pointer
         border bg-card transition-shadow hover:shadow-md
         {isActive ? 'border-primary ring-2 ring-primary/30' : 'border-border'}"
  style="width: 380px; max-height: 60vh;"
  role="button"
  tabindex="0"
  onclick={onclick}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onclick?.(); }}
>
  <div class="flex items-center justify-between px-4 py-2 border-b border-border">
    <span class="text-xs font-semibold text-foreground">v{versionNumber}</span>
    <span class="text-xs text-muted-foreground truncate ml-2">{modelName}</span>
  </div>
  <div class="p-3 flex-1 overflow-y-auto text-sm text-foreground">
    {#if isLoading}
      <div class="space-y-2 animate-pulse">
        <div class="h-3 bg-muted rounded w-3/4"></div>
        <div class="h-3 bg-muted rounded w-full"></div>
        <div class="h-3 bg-muted rounded w-5/6"></div>
      </div>
    {:else}
      <div class="message-markdown">{@html renderedContent}</div>
    {/if}
  </div>
</div>
```

---

## 4. 交互流程设计

### 4.1 用户旅程图

```
用户看到 AI 回复（有多版本标记 v1/v2/v3）
    |
    v
用户点击 [<->] 对比按钮
    |
    v
面板 slide-down 展开，显示 loading 骨架卡片
    |
    v
api.listVersions() 返回版本列表
    |
    +-----> 失败：显示错误提示 + 重试按钮
    |
    v 成功
逐个加载/获取每个版本的 Message 内容
    |
    v
卡片逐个显示内容（当前版本卡片高亮边框）
    |
    +-----> 用户横向滚动查看不同版本
    |
    +-----> 用户点击某张卡片 -> 调用 switchVersion()
    |       -> 当前版本更新，高亮边框移动到新卡片
    |       -> 对比面板保持打开
    |
    +-----> 用户再次点击 [<->] 按钮
    |
    v
面板 slide-up 收起，恢复单版本显示
```

### 4.2 状态转换表

| 当前状态 | 触发事件 | 下一状态 | UI 变化 |
|----------|----------|----------|---------|
| Collapsed | 点击对比按钮 | Loading | 面板 slide-down 展开，显示骨架卡片 |
| Loading | 数据加载成功 | Expanded | 骨架卡片替换为实际内容卡片 |
| Loading | 数据加载失败 | Error | 显示错误消息和重试按钮 |
| Error | 点击重试 | Loading | 重新请求数据 |
| Expanded | 点击某张卡片 | Expanded | 高亮边框移动到新卡片，触发 switchVersion |
| Expanded | 点击对比按钮 | Collapsed | 面板 slide-up 收起 |
| Expanded | 横向滚动 | Expanded | 滚动容器展示更多卡片 |
| 任意 | 消息被删除/对话切换 | Collapsed | 面板强制收起 |

### 4.3 关键交互细节

#### 交互 1：展开/收起动画

- **动画方式**：CSS `transition` + Svelte `{#if}` 或 `slide` transition
- **推荐实现**：使用 Svelte 内置 `slide` transition

```svelte
{#if isCompareOpen}
  <div transition:slide={{ duration: 200 }}>
    <VersionComparePanel ... />
  </div>
{/if}
```

- **展开时机**：点击对比按钮后立即开始展开动画，同时发起数据请求
- **收起时机**：点击对比按钮或对话切换时收起

#### 交互 2：横向滚动

- **滚动容器**：对比面板的 flex 容器，`overflow-x: auto`
- **滚动条样式**：`scrollbar-width: thin`，与全局滚动条风格一致
- **初始滚动位置**：展开时自动将当前选中版本的卡片滚动到视口可见区域
- **实现方式**：

```typescript
function scrollToActiveCard(container: HTMLDivElement, versionNumber: number) {
  const activeCard = container.querySelector(`[data-version="${versionNumber}"]`);
  activeCard?.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' });
}
```

#### 交互 3：卡片点击切换版本

- 点击卡片触发 `onSwitchVersion(versionNumber)`
- 通过 `onAction?.({ type: 'switchVersion', ... })` 冒泡到 MessageList -> 聊天页面
- 对比面板保持打开状态，高亮框移动到新卡片
- 面板下方的单版本内容区隐藏中（对比面板展开时隐藏）

#### 交互 4：对比面板与单版本内容互斥

- `isCompareOpen === true` 时：显示对比面板，隐藏单版本内容（markdown 渲染、reasoning 区域）
- `isCompareOpen === false` 时：隐藏对比面板，显示单版本内容（恢复现状）
- 版本按钮组始终可见（不受展开/收起影响）

#### 交互 5：键盘导航

- 对比按钮可通过 Tab 聚焦，Enter/Space 触发展开/收起
- 各卡片可通过 Tab 聚焦，Enter/Space 触发切换版本
- 面板内左右箭头键可切换聚焦的卡片

---

## 5. MessageBubble.svelte 修改方案

### 5.1 新增状态变量

```typescript
let isCompareOpen = $state(false);
let compareVersions: VersionInfo[] = $state([]);
let compareMessages: Message[] = $state([]);
let isCompareLoading = $state(false);
let compareError = $state('');
```

### 5.2 新增方法

```typescript
import Columns2Icon from '@lucide/svelte/icons/columns-2';
import { slide } from 'svelte/transition';

async function toggleCompare() {
  if (isCompareOpen) {
    isCompareOpen = false;
    return;
  }

  isCompareOpen = true;
  isCompareLoading = true;
  compareError = '';

  try {
    const groupId = message.versionGroupId || message.id;
    const versions = await api.listVersions(groupId);
    compareVersions = versions;

    // 加载所有版本的消息内容
    // 需要后端提供 getVersionMessages 或 getMessageById API
    const msgs = await api.getVersionMessages(groupId);
    compareMessages = msgs;
  } catch (e) {
    compareError = String(e);
  } finally {
    isCompareLoading = false;
  }
}
```

### 5.3 模板修改位置

在版本按钮组 `{#each}` 循环后面插入对比按钮：

```svelte
{#if message.totalVersions > 1}
  <div class="flex items-center gap-1 flex-wrap">
    {#each Array.from({ length: message.totalVersions }, (_, i) => i + 1) as v}
      <button
        class="rounded px-2 py-0.5 text-xs cursor-pointer {v === message.versionNumber
          ? 'bg-foreground text-background font-medium'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
        onclick={() => switchVersion(v)}
      >
        v{v}
      </button>
    {/each}

    <!-- NEW: 对比按钮 -->
    <button
      class="rounded p-1 ml-1 cursor-pointer {isCompareOpen
        ? 'bg-foreground text-background'
        : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
      title={isCompareOpen ? i18n.t.collapseCompare : i18n.t.expandCompare}
      onclick={() => void toggleCompare()}
    >
      <Columns2Icon size={14} />
    </button>
  </div>

  <!-- NEW: 对比面板 -->
  {#if isCompareOpen}
    <div transition:slide={{ duration: 200 }}>
      <VersionComparePanel
        versions={compareVersions}
        messages={compareMessages}
        currentVersionNumber={message.versionNumber}
        {isCompareLoading}
        error={compareError}
        totalVersions={message.totalVersions}
        onSwitchVersion={(v) => switchVersion(v)}
        onRetry={() => void toggleCompare()}
      />
    </div>
  {/if}
{/if}

<!-- 对比面板展开时隐藏单版本内容 -->
{#if !isCompareOpen}
  {#if isLoading}
    <!-- ... existing loading indicator ... -->
  {/if}

  {#if message.reasoning}
    <!-- ... existing reasoning section ... -->
  {/if}

  {#if message.content}
    <!-- ... existing content section ... -->
  {/if}
{/if}
```

---

## 6. 新增 i18n 词条

在 `src/lib/stores/i18n.svelte.ts` 中新增：

```typescript
// zh
expandCompare: '展开对比',
collapseCompare: '收起对比',
compareLoadError: '加载版本对比失败',
retry: '重试',

// en
expandCompare: 'Compare versions',
collapseCompare: 'Collapse compare',
compareLoadError: 'Failed to load version comparison',
retry: 'Retry',
```

---

## 7. 新增后端 API（建议）

在 `src/lib/utils/invoke.ts` 中新增：

```typescript
getVersionMessages(versionGroupId: string): Promise<Message[]> {
  return invoke('get_version_messages', { versionGroupId });
}
```

对应 Rust 端新增 `get_version_messages` 命令，输入 `versionGroupId`，返回该版本组所有版本的完整 `Message` 列表（按 `versionNumber` 排序）。

如果后端暂不新增接口，前端可用以下降级方案：

```typescript
async function loadCompareMessages(groupId: string): Promise<Message[]> {
  const versions = await api.listVersions(groupId);
  // listVersions 返回 VersionInfo[]，包含 id (即 messageId)
  // 暂时使用 getMessages 无法按 id 精确获取，需后端支持
  // 降级方案：前端只展示当前已有的 message + 版本元信息
}
```

---

## 8. 模型名称解析

卡片 header 需要展示模型名称。`VersionInfo` 中有 `modelId`，需要将其映射为可读名称。

**方案**：复用已有 `api.getVersionModels(versionGroupId)` 返回的 `[versionNumber, modelId][]`，结合全局 model 列表进行名称解析。

如果无法解析（模型已删除），则直接显示 `modelId` 截断展示，或显示 "Unknown Model"。

---

## 9. 无障碍访问（A11y）

| 实践 | 实施方法 |
|------|----------|
| 对比按钮语义 | `<button>` 原生按钮，`title` 描述当前操作 |
| 卡片可访问 | `role="button"`, `tabindex="0"`, Enter/Space 触发 |
| 当前选中指示 | `aria-pressed="true"` 或 `aria-current="true"` 在活跃卡片上 |
| 面板展开/收起 | 对比按钮使用 `aria-expanded={isCompareOpen}` |
| 横向滚动容器 | `role="region"`, `aria-label="版本对比面板"` |
| 焦点管理 | 展开时自动聚焦到当前版本卡片 |

---

## 10. 对 MessageList 虚拟化的影响

**关键注意点**：`MessageList` 使用虚拟滚动，依赖 `ResizeObserver` 测量每个 `message-row` 的高度。

- 对比面板展开后，`message-row` 的高度会显著变化（从单版本内容高度变为面板高度）
- 现有的 `measureRow` + `ResizeObserver` 机制会**自动检测**高度变化并更新 `heightCache`
- 不需要额外修改虚拟化逻辑
- 但需注意：展开面板后如果 `shouldFollowBottom === true`，会触发自动滚动到底部。这在展开中间消息的对比面板时可能不符合预期。可考虑在展开时临时禁止 follow-bottom 行为。

---

## 11. 文件修改清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/lib/components/chat/MessageBubble.svelte` | **修改** | 新增对比按钮 + 对比面板展开逻辑 |
| `src/lib/components/chat/VersionComparePanel.svelte` | **新建** | 横向滚动对比容器 |
| `src/lib/components/chat/VersionCard.svelte` | **新建** | 单版本卡片组件 |
| `src/lib/stores/i18n.svelte.ts` | **修改** | 新增对比相关 i18n 词条 |
| `src/lib/utils/invoke.ts` | **修改** | 新增 `getVersionMessages()` API 调用 |
| Rust 后端 | **修改** | 新增 `get_version_messages` 命令 |

---

## 12. 组件间数据流

```
MessageBubble
  |
  |-- 点击对比按钮 --> toggleCompare()
  |     |
  |     +--> api.listVersions(groupId)     --> VersionInfo[]
  |     +--> api.getVersionMessages(groupId) --> Message[]
  |     |
  |     v
  |   VersionComparePanel (props: versions, messages, currentVersionNumber)
  |     |
  |     +--> VersionCard x N (props: content, modelName, isActive, ...)
  |           |
  |           +--> 点击卡片 --> onSwitchVersion(v)
  |                              |
  |                              v
  |                         switchVersion(v) --> onAction({type:'switchVersion',...})
  |                              |
  |                              v
  |                     MessageList --> 父组件处理版本切换
  |                              |
  |                              v
  |                     message prop 更新 --> 高亮卡片变更
```

---

## 13. 视觉设计细节

### 13.1 对比按钮与版本按钮的视觉关系

对比按钮与 `v1/v2/v3` 按钮同行，但通过 `ml-1` 略微与版本按钮分隔，暗示其为不同类型的操作（展开面板 vs 切换版本）。可以加一个细微的竖线分隔符：

```svelte
<span class="w-px h-4 bg-border mx-1"></span>
<button ...>
  <Columns2Icon size={14} />
</button>
```

### 13.2 卡片高亮边框

当前选中版本的卡片使用 `ring` 而非 `border` 变化，避免宽度跳动：

```
非选中: border border-border
选中:   border border-primary ring-2 ring-primary/20
```

### 13.3 卡片内滚动条

每张卡片的 body 区域有独立纵向滚动：

```css
.card-body {
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: oklch(0.85 0 0) transparent;
}
```

### 13.4 对比面板最大高度

面板自身不设固定高度，但通过 `max-height: 60vh` 约束每张卡片的最大高度，防止面板过高挤占整个聊天视口。

### 13.5 面板外层样式

面板外层添加微弱的顶部边框或背景区分，表明这是一个特殊的展开区域：

```svelte
<div class="rounded-lg border border-border bg-muted/30 p-2">
  <div class="flex gap-3 overflow-x-auto ...">
    ...cards...
  </div>
</div>
```
