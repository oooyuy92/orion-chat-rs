// @ts-nocheck
import test from 'node:test';
import assert from 'node:assert/strict';
import { createAutoUpdaterController } from '../../utils/autoUpdater.js';

test('returns up-to-date when no update is available', async () => {
  const controller = createAutoUpdaterController({
    releaseUrl: 'https://example.com/release',
    check: async () => null,
    relaunch: async () => {},
  });

  const result = await controller.checkForUpdates({ autoDownload: true });

  assert.equal(result.phase, 'up-to-date');
  assert.equal(result.releaseUrl, 'https://example.com/release');
});

test('returns available without downloading when autoDownload is disabled', async () => {
  let downloaded = false;
  const controller = createAutoUpdaterController({
    releaseUrl: 'https://example.com/release',
    check: async () => ({
      currentVersion: '0.1.0',
      version: '0.2.0',
      download: async () => {
        downloaded = true;
      },
      install: async () => {},
    }),
    relaunch: async () => {},
  });

  const result = await controller.checkForUpdates({ autoDownload: false });

  assert.equal(result.phase, 'available');
  assert.equal(result.version, '0.2.0');
  assert.equal(downloaded, false);
});

test('downloads a pending update on demand after manual check', async () => {
  let downloaded = false;
  const controller = createAutoUpdaterController({
    releaseUrl: 'https://example.com/release',
    check: async () => ({
      currentVersion: '0.1.0',
      version: '0.2.0',
      download: async () => {
        downloaded = true;
      },
      install: async () => {},
    }),
    relaunch: async () => {},
  });

  await controller.checkForUpdates({ autoDownload: false });
  const result = await controller.downloadPendingUpdate();

  assert.equal(result.phase, 'downloaded');
  assert.equal(downloaded, true);
});

test('auto downloads and becomes downloaded when update is available', async () => {
  const events = [];
  let installed = false;
  let relaunched = false;
  const controller = createAutoUpdaterController({
    releaseUrl: 'https://example.com/release',
    check: async () => ({
      currentVersion: '0.1.0',
      version: '0.2.0',
      download: async (onEvent) => {
        onEvent?.({ event: 'Started', data: { contentLength: 100 } });
        onEvent?.({ event: 'Progress', data: { chunkLength: 40 } });
        onEvent?.({ event: 'Progress', data: { chunkLength: 60 } });
        onEvent?.({ event: 'Finished' });
      },
      install: async () => {
        installed = true;
      },
    }),
    relaunch: async () => {
      relaunched = true;
    },
  });

  const result = await controller.checkForUpdates({
    autoDownload: true,
    onProgress: (event, snapshot) => events.push([event.event, snapshot.phase, snapshot.downloadedBytes ?? 0]),
  });

  assert.equal(result.phase, 'downloaded');
  assert.equal(result.version, '0.2.0');
  assert.equal(result.downloadedBytes, 100);
  assert.deepEqual(events, [
    ['Started', 'downloading', 0],
    ['Progress', 'downloading', 40],
    ['Progress', 'downloading', 100],
    ['Finished', 'downloading', 100],
  ]);

  const installResult = await controller.installAndRestart();
  assert.equal(installResult.phase, 'installing');
  assert.equal(installed, true);
  assert.equal(relaunched, true);
});

test('surfaces download errors as error state', async () => {
  const controller = createAutoUpdaterController({
    releaseUrl: 'https://example.com/release',
    check: async () => ({
      currentVersion: '0.1.0',
      version: '0.2.0',
      download: async () => {
        throw new Error('network failed');
      },
      install: async () => {},
    }),
    relaunch: async () => {},
  });

  const result = await controller.checkForUpdates({ autoDownload: true });

  assert.equal(result.phase, 'error');
  assert.match(result.error, /network failed/);
});
