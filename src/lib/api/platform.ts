/**
 * 检测当前是否运行在 Tauri 环境中。
 * Web 模式（Docker 部署）时返回 false。
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * 等待 Tauri 平台就绪。
 * 在应用冷启动时,Tauri 的 __TAURI_INTERNALS__ 可能需要几毫秒才能初始化。
 * 此函数会轮询等待,直到 Tauri 就绪或超时。
 *
 * @param options - 配置选项
 * @param options.intervalMs - 轮询间隔(毫秒),默认 1ms
 * @param options.timeoutMs - 超时时间(毫秒),默认 50ms
 * @returns Promise<boolean> - true 表示 Tauri 已就绪,false 表示超时
 */
export async function waitForTauriReady(options?: {
  intervalMs?: number;
  timeoutMs?: number;
}): Promise<boolean> {
  const { intervalMs = 1, timeoutMs = 50 } = options ?? {};

  // 如果已经就绪,立即返回
  if (isTauri()) {
    return true;
  }

  // 轮询等待 Tauri 就绪
  const startTime = Date.now();
  while (Date.now() - startTime < timeoutMs) {
    if (isTauri()) {
      return true;
    }
    await new Promise(resolve => setTimeout(resolve, intervalMs));
  }

  // 超时返回 false (不抛出异常)
  return false;
}
