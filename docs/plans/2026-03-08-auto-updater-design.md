# Auto Updater Design

## Background

当前项目虽然已经有 GitHub Release 工作流和 `v0.2.0` release，但应用内“检查更新”仍然是占位实现：
- 设置页 `handleCheckUpdate()` 只等待 1.2 秒后把状态写成 `latest`
- Tauri 配置与 Rust 包版本仍停留在 `0.1.0`
- 仓库尚未接入 `tauri-plugin-updater`
- Release workflow 的 `includeUpdaterJson` 目前为 `false`

因此，用户在 `0.1.0` 版本里点击“检查更新”，即使 GitHub 已存在 `v0.2.0`，界面仍会显示“已是最新版本”。

## Goal

为 Orion Chat 接入基于 GitHub Releases 的 Tauri 自动更新能力，实现：
- 应用内真实检查更新
- 发现新版本后自动后台下载
- 下载完成后提示用户“重启安装”
- 继续复用现有 GitHub Actions 与 GitHub Releases 发布链路

## Non-Goals

本次不做：
- 自建更新服务器
- 静默强制安装
- 多更新通道（stable/beta/nightly）
- 差分更新策略定制

## Requirements

### Functional

1. 设置页“检查更新”必须发起真实更新检查，而不是本地假状态。
2. 若存在更新且用户开启“自动更新”，则开始后台下载更新包。
3. 下载完成后提示用户确认重启并安装。
4. 若没有更新，则明确显示“已是最新版本”。
5. 若更新检查或下载失败，则显示错误信息，并保留跳转 GitHub release 页作为兜底。
6. 应用内显示的当前版本必须与实际应用打包版本一致。

### Release/Operations

1. GitHub Release workflow 必须生成 Tauri updater 所需元数据。
2. 更新包必须由 updater 私钥签名。
3. GitHub Secrets 中需要配置 updater 私钥（以及必要的密码，如果使用）。
4. 仓库文档中需要写明后续发版时的要求与依赖。

## Recommended Approach

采用 **Tauri 官方 updater + GitHub Releases** 标准方案：

- **应用端**：使用 `tauri-plugin-updater` 执行检查、下载、安装
- **发布端**：使用现有 `tauri-apps/tauri-action`，开启 updater 元数据生成与上传
- **版本源**：继续使用 GitHub Releases 作为更新源
- **交互策略**：检查到新版本后自动后台下载，下载完成再提示重启安装

### Why this approach

1. 与当前发布链路兼容，增量最小。
2. Tauri 官方能力维护成本最低。
3. 不需要额外部署更新服务。
4. 用户体验与桌面应用预期一致。

## Alternatives Considered

### Option A: 仅检查版本并跳转下载页

优点：实现快，风险低。
缺点：不是完整自动更新，无法满足本次需求。

### Option B: 自建更新元数据服务

优点：可控性更高，后续支持灰度/渠道更灵活。
缺点：复杂度和运维成本明显高于当前需要。

## Architecture

### 1. Version Source of Truth

需要统一以下版本来源：
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- Git tag / GitHub Release（如 `v0.2.0`）

建议规则：
- 应用内部版本使用不带 `v` 的语义化版本，如 `0.2.0`
- Git tag 使用 `v0.2.0`
- Release workflow 以 tag 为输入，但构建出的应用版本必须已经与 tag 对齐

### 2. App-side Update Flow

应用端需要一个清晰的更新状态机：
- `idle`
- `checking`
- `up-to-date`
- `available`
- `downloading`
- `downloaded`
- `error`

推荐行为：
1. 用户点击“检查更新” → 状态变为 `checking`
2. 若无更新 → `up-to-date`
3. 若有更新：
   - 若 `autoUpdate = true` → 自动进入 `downloading`
   - 若 `autoUpdate = false` → 提示发现更新但不自动下载
4. 下载完成 → `downloaded`
5. 用户确认重启 → 调用安装并重启

### 3. Backend Integration

Tauri 端接入 `tauri-plugin-updater`。

实现方式建议优先采用一层轻薄的 Rust 命令封装，而不是让前端直接深度持有插件对象：
- `check_for_update`
- `download_update`
- `install_update`
- 可选：返回当前状态/可用版本信息

这样做的好处：
- 前端接口稳定，便于测试
- 后续若要更换更新策略，前端改动更小
- 错误信息和平台差异可在后端统一处理

### 4. Frontend Integration

设置页 About 面板需要从占位实现改成真实逻辑：
- 显示当前版本号
- 显示更新状态文案
- 处理自动下载开关
- 下载完成后弹确认框或显示安装按钮
- 失败时显示错误和 release 页链接

为避免把更新逻辑塞满 `ProviderSettings.svelte`，建议把更新相关调用收敛到 `src/lib/utils/invoke.ts` 或新增一个轻量 helper/store。

## Release Workflow Changes

当前 workflow 使用 `tauri-action` 发布产物，但未生成 updater 元数据。

需要变更：
1. 开启 updater 元数据生成
2. 在 GitHub Actions 中注入 updater 私钥相关 secrets
3. 确认 release 资产包含 updater 所需文件
4. release body 文案从“首个公开版本”改为通用更新说明

## Security Requirements

自动更新必须建立在签名校验上，因此需要：
- 生成 updater 签名私钥 / 公钥
- 将私钥放入 GitHub Secrets
- 将公钥写入应用配置
- 应用只接受由匹配私钥签名的更新包

如果密钥缺失：
- 构建仍可能成功
- 但自动更新链路不会成为真正可用状态

因此本次设计默认：**代码与 workflow 接好，同时要求补齐 Secrets 配置**。

## Error Handling

### Check failures
- 网络失败
- GitHub release 不可访问
- 元数据缺失或格式错误

处理方式：
- UI 显示明确错误
- 保留重试按钮
- 提供“打开 release 页面”兜底入口

### Download failures
- 网络中断
- 签名验证失败
- 资产缺失

处理方式：
- 状态切换到 `error`
- 显示失败原因
- 允许重新检查/重新下载

### Version mismatch
- App version 仍为 `0.1.0`
- 最新 release 为 `v0.2.0`

处理方式：
- 统一版本配置
- 在发版前验证 `Cargo.toml`、`tauri.conf.json` 与 tag 一致

## Testing Strategy

### Automated

1. 纯前端状态逻辑测试：
   - 检查中 → 最新版本
   - 检查中 → 有更新 → 自动下载
   - 下载完成 → 等待安装
   - 失败 → 错误状态
2. Rust 侧命令测试：
   - 没有可用更新时的返回结构
   - 错误传播格式
3. 静态配置契约测试：
   - updater plugin 已注册
   - workflow 已开启 updater 元数据
   - 版本号配置已更新到目标版本

### Manual

1. 用旧版本安装包启动应用
2. 发布一个更高版本 release
3. 在设置页点击“检查更新”
4. 观察后台下载状态
5. 下载完成后确认“重启安装”
6. 重启后确认版本号变更

## Risks

1. **GitHub Release 已创建但 assets 尚未上传完成**
   - 检查更新可能早于资产完整可用
   - 需要在 UI 中提供可重试策略
2. **签名密钥未配置**
   - 自动更新看似接通，实际不可用
3. **版本号不同步**
   - 会继续出现“明明有新版本却识别错误”的问题
4. **设置页组件过重**
   - 更新逻辑直接写在 `ProviderSettings.svelte` 中会继续加重耦合

## Success Criteria

满足以下条件可视为完成：
- `0.1.x` 版本客户端能检测到 `0.2.0+` release
- 检查更新不再是假状态
- 自动后台下载可触发
- 下载完成后能提示并执行重启安装
- GitHub Release workflow 能产出 updater 所需资产
- 应用版本号、Git tag、GitHub Release 三者保持一致
