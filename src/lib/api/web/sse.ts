/**
 * Web 端 SSE 流式请求工具。
 * 对应 Tauri 端的 Channel<ChatEvent>。
 */
import type { ChatEvent } from '$lib/utils/invoke';

export async function streamRequest(
  url: string,
  body: unknown,
  onEvent: (event: ChatEvent) => void,
  signal?: AbortSignal
): Promise<void> {
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
    signal,
  });

  if (!response.ok) {
    const err = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(err.error ?? response.statusText);
  }

  const reader = response.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';
    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = line.slice(6).trim();
        if (data === '[DONE]') return;
        try {
          onEvent(JSON.parse(data) as ChatEvent);
        } catch {
          // ignore malformed lines
        }
      }
    }
  }
}
