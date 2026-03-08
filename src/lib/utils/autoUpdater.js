/**
 * @typedef {'idle' | 'checking' | 'up-to-date' | 'available' | 'downloading' | 'downloaded' | 'installing' | 'error'} AutoUpdaterPhase
 */

/**
 * @typedef {{
 *   phase: AutoUpdaterPhase;
 *   releaseUrl: string;
 *   currentVersion?: string;
 *   version?: string;
 *   downloadedBytes?: number;
 *   totalBytes?: number | null;
 *   error?: string;
 * }} AutoUpdaterSnapshot
 */

/**
 * @typedef {{
 *   currentVersion: string;
 *   version: string;
 *   body?: string;
 *   date?: string;
 *   download: (onEvent?: (event: any) => void) => Promise<void>;
 *   install: () => Promise<void>;
 *   close?: () => Promise<void>;
 * }} UpdateLike
 */

/**
 * @param {{
 *   releaseUrl: string;
 *   check: () => Promise<UpdateLike | null>;
 *   relaunch: () => Promise<void>;
 * }} driver
 */
export function createAutoUpdaterController({ releaseUrl, check, relaunch }) {
  /** @type {UpdateLike | null} */
  let pendingUpdate = null;
  /** @type {AutoUpdaterSnapshot} */
  let snapshot = { phase: 'idle', releaseUrl };

  async function closePendingUpdate() {
    if (pendingUpdate?.close) {
      await pendingUpdate.close();
    }
    pendingUpdate = null;
  }

  /** @param {Partial<AutoUpdaterSnapshot>} next */
  function updateSnapshot(next) {
    snapshot = {
      ...snapshot,
      ...next,
      releaseUrl,
    };
    return { ...snapshot };
  }

  /**
   * @param {{ onProgress?: (event: any, snapshot: AutoUpdaterSnapshot) => void }} [options]
   */
  async function downloadPendingUpdate(options = {}) {
    if (!pendingUpdate) {
      return updateSnapshot({
        phase: 'error',
        error: 'No pending update available.',
      });
    }

    let downloadedBytes = 0;
    let totalBytes = snapshot.totalBytes ?? null;
    updateSnapshot({ phase: 'downloading', downloadedBytes: 0, totalBytes, error: undefined });

    try {
      await pendingUpdate.download((event) => {
        if (event.event === 'Started') {
          totalBytes = event.data.contentLength ?? null;
          const next = updateSnapshot({ phase: 'downloading', downloadedBytes, totalBytes });
          options.onProgress?.(event, next);
          return;
        }

        if (event.event === 'Progress') {
          downloadedBytes += event.data.chunkLength;
          const next = updateSnapshot({ phase: 'downloading', downloadedBytes, totalBytes });
          options.onProgress?.(event, next);
          return;
        }

        const next = updateSnapshot({ phase: 'downloading', downloadedBytes, totalBytes });
        options.onProgress?.(event, next);
      });

      return updateSnapshot({
        phase: 'downloaded',
        downloadedBytes,
        totalBytes,
        error: undefined,
      });
    } catch (error) {
      await closePendingUpdate();
      return updateSnapshot({
        phase: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  /**
   * @param {{ autoDownload?: boolean; onProgress?: (event: any, snapshot: AutoUpdaterSnapshot) => void }} [options]
   */
  async function checkForUpdates(options = {}) {
    await closePendingUpdate();
    updateSnapshot({
      phase: 'checking',
      currentVersion: undefined,
      version: undefined,
      downloadedBytes: undefined,
      totalBytes: undefined,
      error: undefined,
    });

    try {
      const update = await check();
      if (!update) {
        return updateSnapshot({ phase: 'up-to-date' });
      }

      pendingUpdate = update;
      updateSnapshot({
        phase: 'available',
        currentVersion: update.currentVersion,
        version: update.version,
        downloadedBytes: 0,
        totalBytes: null,
      });

      if (options.autoDownload === false) {
        return { ...snapshot };
      }

      return downloadPendingUpdate({ onProgress: options.onProgress });
    } catch (error) {
      await closePendingUpdate();
      return updateSnapshot({
        phase: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  async function installAndRestart() {
    if (!pendingUpdate) {
      return updateSnapshot({
        phase: 'error',
        error: 'No downloaded update available.',
      });
    }

    try {
      const next = updateSnapshot({ phase: 'installing', error: undefined });
      await pendingUpdate.install();
      await relaunch();
      return next;
    } catch (error) {
      return updateSnapshot({
        phase: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return {
    checkForUpdates,
    downloadPendingUpdate,
    installAndRestart,
    getSnapshot() {
      return { ...snapshot };
    },
  };
}

/**
 * @param {{ releaseUrl?: string }} [options]
 */
export async function createDefaultAutoUpdaterController(options = {}) {
  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ]);

  return createAutoUpdaterController({
    releaseUrl: options.releaseUrl ?? 'https://github.com/oooyuy92/orion-chat-rs/releases/latest',
    check,
    relaunch,
  });
}
