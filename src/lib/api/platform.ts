/**
 * 检测当前是否运行在 Tauri 环境中。
 * Web 模式（Docker 部署）时返回 false。
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
