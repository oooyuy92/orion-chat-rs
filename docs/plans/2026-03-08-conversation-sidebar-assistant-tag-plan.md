# Conversation Sidebar Assistant Tag Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在左栏会话列表中，为绑定了 Assistant 的会话在标题下方显示 `#助手名`。

**Architecture:** 保持后端接口不变，在前端侧栏组件中额外拉取 Assistant 列表并建立 id 到名称的映射。渲染层为会话标题新增一行次级标签，仅在存在有效 Assistant 绑定时显示。

**Tech Stack:** Svelte 5、TypeScript、Node 内置 `node:test` 契约测试、`svelte-check`

---

### Task 1: 侧栏契约测试

**Files:**
- Create: `src/lib/components/sidebar/conversationAssistantTag.test.js`
- Test: `src/lib/components/sidebar/conversationAssistantTag.test.js`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ConversationList.svelte', import.meta.url), 'utf8');

test('conversation list loads assistants for sidebar labels', () => {
  assert.match(source, /api\.listAssistants\(/);
});

test('conversation list renders assistant label under title', () => {
  assert.match(source, /#\{assistantNameFor\(conversation\)\}/);
});
```

**Step 2: Run test to verify it fails**

Run: `node --test src/lib/components/sidebar/conversationAssistantTag.test.js`
Expected: FAIL，因为组件还没有加载 assistants，也没有渲染 `#助手名`。

**Step 3: Write minimal implementation**

```svelte
<script lang="ts">
  let assistants = $state<Assistant[]>([]);
  function assistantNameFor(conversation: Conversation) {
    return assistants.find((a) => a.id === conversation.assistantId)?.name ?? '';
  }
</script>
```

并在普通会话渲染态中加入第二行标签。

**Step 4: Run test to verify it passes**

Run: `node --test src/lib/components/sidebar/conversationAssistantTag.test.js`
Expected: PASS

### Task 2: 侧栏渲染与样式收口

**Files:**
- Modify: `src/lib/components/sidebar/ConversationList.svelte`
- Test: `src/lib/components/sidebar/conversationAssistantTag.test.js`

**Step 1: Keep layout compact**
- 为会话按钮改成适配两行内容的布局。
- 标题行保留前缀与置顶图标。
- Assistant 标签使用更小字号、次级颜色、单行截断。

**Step 2: Hide tag when unavailable**
- 无 `assistantId` 或找不到 Assistant 名称时不渲染第二行。
- 输入态（rename / prefix）保持当前单行结构。

**Step 3: Verify implementation**

Run: `node --test src/lib/components/sidebar/conversationAssistantTag.test.js`
Expected: PASS

Run: `pnpm run check`
Expected: `svelte-check found 0 errors and 0 warnings`
