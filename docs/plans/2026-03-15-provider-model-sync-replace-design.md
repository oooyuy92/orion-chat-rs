# Provider Model Sync Replace Design

**背景**
- 当前 provider 模型同步使用 `INSERT ... ON CONFLICT(id) DO UPDATE` 逐条 upsert。
- 同步结束后，后端返回该 provider 在数据库中的全部模型，而不是本次远端返回集合。
- 结果是旧的 `synced` 模型不会被清理，设置页和模型选择器会持续累积历史同步结果。

**目标**
- 每次同步后，`synced` 模型集合以本次远端返回结果为准。
- `manual` 模型继续保留，不参与覆盖。
- 对仍被 `assistants` / `conversations` 引用的旧 `synced` 模型，先保留，避免直接删除造成引用错误。

## 最终方案
- 新增一个数据库协调函数，负责“替换某个 provider 的 `synced` 模型集合”。
- 流程：
  1. 对本次远端模型执行 upsert。
  2. 找出该 provider 下旧的 `synced` 模型中“不在本次远端结果里”的项。
  3. 删除其中未被 `assistants` 或 `conversations` 引用的项。
  4. 返回该 provider 的最新模型列表。

## 取舍
- 不直接删除所有旧 `synced` 模型：
  - 因为 `assistants.model_id` 和 `conversations.model_id` 对 `models` 有外键引用。
  - 直接删会把已有配置打断，风险过高。
- 这意味着“已被引用的过期同步模型”仍会临时保留。
- 但相比当前无限累积，这已经把问题收敛到“只有被引用的历史模型保留”。

## 验证
- 新增 Rust 测试覆盖：
  - 同步后未返回的旧 `synced` 模型会被清理
  - `manual` 模型不会被清理
  - 被 `conversation` / `assistant` 引用的旧 `synced` 模型不会被清理
