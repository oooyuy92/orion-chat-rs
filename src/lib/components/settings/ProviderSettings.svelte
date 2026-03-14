<script lang="ts">
  import { onMount, tick } from 'svelte';
  import type { ProviderConfig, ProviderType } from '$lib/types';
  import { api } from '$lib/utils/invoke';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Slider } from '$lib/components/ui/slider';
  import AssistantSettings from '$lib/components/settings/AssistantSettings.svelte';
  import { loadCombos, saveCombos, addCombo, deleteCombo } from '$lib/stores/modelCombos';
  import type { ModelCombo } from '$lib/types';
  import { isManualModel, resolveModelLabel, resolveModelSecondaryLabel } from '$lib/utils/modelDisplay';
  import { load as loadStore } from '@tauri-apps/plugin-store';
  import { getVersion } from '@tauri-apps/api/app';
  import { i18n, type Language } from '$lib/stores/i18n.svelte';
  import { createDefaultAutoUpdaterController } from '$lib/utils/autoUpdater.js';

  type NavItemId =
    | 'modelService'
    | 'assistants'
    | 'modelCombos'
    | 'generalSettings'
    | 'displaySettings'
    | 'dataSettings'
    | 'networkSearch'
    | 'globalMemory'
    | 'quickPhrases'
    | 'shortcuts'
    | 'about';

  type SectionGroup = {
    title: string;
    items: NavItemId[];
  };

  const language = $derived(i18n.language as Language);

  function navLabel(item: NavItemId) {
    const isEn = language === 'en';
    switch (item) {
      case 'modelService':
        return isEn ? 'Model Service' : '模型服务';
      case 'assistants':
        return isEn ? 'Assistant Settings' : '助手设置';
      case 'modelCombos':
        return isEn ? 'Model Combos' : '模型组合';
      case 'generalSettings':
        return isEn ? 'General Settings' : '常规设置';
      case 'displaySettings':
        return isEn ? 'Display Settings' : '显示设置';
      case 'dataSettings':
        return isEn ? 'Data Settings' : '数据设置';
      case 'networkSearch':
        return isEn ? 'Web Search' : '网络搜索';
      case 'globalMemory':
        return isEn ? 'Global Memory' : '全局记忆';
      case 'quickPhrases':
        return isEn ? 'Quick Phrases' : '快捷短语';
      case 'shortcuts':
        return isEn ? 'Shortcuts' : '快捷键';
      case 'about':
        return isEn ? 'About' : '关于我们';
    }
  }

  const sectionGroups = $derived.by((): SectionGroup[] => [
    { title: navLabel('modelService'), items: ['modelService', 'assistants', 'modelCombos', 'generalSettings', 'displaySettings', 'dataSettings'] },
    { title: language === 'en' ? 'MCP Servers' : 'MCP 服务器', items: ['networkSearch', 'globalMemory', 'quickPhrases', 'shortcuts'] },
    { title: language === 'en' ? 'Other' : '其他', items: ['about'] },
  ]);

  let activeNav = $state<NavItemId>('modelService');

  const providerTypeOptions = $derived.by(() => [
    { value: 'openaiCompat' as const, label: language === 'en' ? 'OpenAI Compatible' : 'OpenAI 兼容', defaultBase: 'https://api.openai.com/v1' },
    { value: 'anthropic' as const, label: 'Anthropic', defaultBase: 'https://api.anthropic.com' },
    { value: 'gemini' as const, label: 'Gemini', defaultBase: 'https://generativelanguage.googleapis.com' },
    { value: 'ollama' as const, label: 'Ollama', defaultBase: 'http://127.0.0.1:11434' },
  ]);

  let providers = $state<ProviderConfig[]>([]);
  let loading = $state(true);
  let error = $state('');
  let success = $state('');

  let selectedProviderId = $state('');
  let search = $state('');

  let draftName = $state('');
  let draftType = $state<ProviderType>('openaiCompat');
  let draftApiBase = $state('');
  let draftApiKey = $state('');
  let draftEnabled = $state(true);

  let showApiKey = $state(false);
  let saving = $state(false);
  let deleting = $state(false);
  let updatingModels = $state<Record<string, boolean>>({});
  let bulkUpdatingModels = $state(false);

  let syncingModels = $state<Record<string, boolean>>({});

  let editingName = $state(false);
  let editNameInput = $state<HTMLInputElement | null>(null);
  let contextMenu = $state<{ x: number; y: number; providerId: string } | null>(null);
  let modelSearch = $state('');
  let showAddModelDialog = $state(false);
  let creatingManualModel = $state(false);
  let manualModelRequestName = $state('');
  let manualModelDisplayName = $state('');
  let manualModelEnabled = $state(true);

  // Model combos state
  let combos = $state<ModelCombo[]>([]);
  let editingComboId = $state<string | null>(null);
  let editComboName = $state('');
  let editComboModelIds = $state<string[]>([]);
  let slotPickerIndex = $state<number | null>(null); // which slot is picking a model (null=closed, -1=adding new)

  const allEnabledModels = $derived(
    providers
      .filter((p) => p.enabled)
      .flatMap((p) => p.models.filter((m) => m.enabled).map((m) => ({ ...m, providerName: p.name })))
  );

  async function loadComboList() {
    combos = await loadCombos();
  }

  function startNewCombo() {
    editingComboId = 'new';
    editComboName = '';
    editComboModelIds = [];
    slotPickerIndex = null;
  }

  function startEditCombo(combo: ModelCombo) {
    editingComboId = combo.id;
    editComboName = combo.name;
    editComboModelIds = [...combo.modelIds];
    slotPickerIndex = null;
  }

  function cancelEditCombo() {
    editingComboId = null;
    slotPickerIndex = null;
  }

  function removeComboModelAt(index: number) {
    editComboModelIds = editComboModelIds.filter((_, i) => i !== index);
    slotPickerIndex = null;
  }

  function openSlotPicker(index: number) {
    // Toggle: if clicking the same slot, close it
    slotPickerIndex = slotPickerIndex === index ? null : index;
  }

  function pickModelForSlot(modelId: string) {
    if (slotPickerIndex === null) return;
    if (slotPickerIndex === -1) {
      // Adding new
      editComboModelIds = [...editComboModelIds, modelId];
    } else {
      // Swapping existing
      editComboModelIds = editComboModelIds.map((id, i) => (i === slotPickerIndex ? modelId : id));
    }
    slotPickerIndex = null;
  }

  function resolveComboModelName(modelId: string): { name: string; providerName: string } {
    for (const m of allEnabledModels) {
      if (m.id === modelId) return { name: m.name, providerName: m.providerName };
    }
    return { name: modelId, providerName: '' };
  }

  async function saveCombo() {
    if (!editComboName.trim() || editComboModelIds.length < 2) return;
    if (editingComboId === 'new') {
      const combo: ModelCombo = {
        id: crypto.randomUUID(),
        name: editComboName.trim(),
        modelIds: editComboModelIds,
      };
      await addCombo(combo);
    } else if (editingComboId) {
      const updated = combos.map((c) =>
        c.id === editingComboId
          ? { ...c, name: editComboName.trim(), modelIds: editComboModelIds }
          : c,
      );
      await saveCombos(updated);
    }
    editingComboId = null;
    slotPickerIndex = null;
    await loadComboList();
  }

  async function handleDeleteCombo(id: string) {
    await deleteCombo(id);
    await loadComboList();
  }

  $effect(() => {
    if (activeNav === 'modelCombos') {
      loadComboList();
    }
  });

  // ── 常规设置 ──
  let proxyMode = $state<'system' | 'none'>('system');
  let autoLaunch = $state(false);

  async function handleProxyChange(mode: 'system' | 'none') {
    proxyMode = mode;
    try { await api.setProxyMode(mode); } catch (e) { console.error(e); }
  }

  async function handleAutoLaunchChange(val: boolean) {
    autoLaunch = val;
    try { await api.setAutostartEnabled(val); } catch (e) { console.error(e); }
  }

  // ── 显示设置 ──
  type ThemeColor = { name: string; nameEn: string; primary: string; primaryForeground: string };

  // Auto-rename
  let autoRename = $state(false);
  let autoRenameModelId = $state('');

  // Auto-compress
  let autoCompress = $state(false);
  let autoCompressModelId = $state('');
  let autoCompressThreshold = $state(50000);

  const allModelGroups = $derived(
    providers
      .filter((p) => p.enabled)
      .map((p) => ({ providerName: p.name, models: p.models.filter((m) => m.enabled) }))
      .filter((g) => g.models.length > 0)
  );

  const themeColors: ThemeColor[] = [
    { name: '石墨黑', nameEn: 'Graphite',   primary: 'oklch(0.205 0 0)',       primaryForeground: 'oklch(0.985 0 0)' },
    { name: '雾霾蓝', nameEn: 'Haze Blue',  primary: 'oklch(0.55 0.08 240)',   primaryForeground: 'oklch(0.98 0 0)' },
    { name: '灰豆绿', nameEn: 'Sage',       primary: 'oklch(0.58 0.07 155)',   primaryForeground: 'oklch(0.98 0 0)' },
    { name: '烟粉色', nameEn: 'Dusty Rose', primary: 'oklch(0.60 0.08 10)',    primaryForeground: 'oklch(0.98 0 0)' },
    { name: '燕麦棕', nameEn: 'Oat Brown',  primary: 'oklch(0.55 0.06 60)',    primaryForeground: 'oklch(0.98 0 0)' },
    { name: '薰衣紫', nameEn: 'Lavender',   primary: 'oklch(0.55 0.09 290)',   primaryForeground: 'oklch(0.98 0 0)' },
    { name: '暖灰色', nameEn: 'Warm Grey',  primary: 'oklch(0.50 0.02 70)',    primaryForeground: 'oklch(0.98 0 0)' },
    { name: '深青色', nameEn: 'Dark Teal',  primary: 'oklch(0.50 0.09 195)',   primaryForeground: 'oklch(0.98 0 0)' },
  ];

  let selectedColorIndex = $state(0);
  let zoomLevel = $state<number>(100);

  function applyThemeColor(index: number) {
    selectedColorIndex = index;
    const color = themeColors[index];
    document.documentElement.style.setProperty('--primary', color.primary);
    document.documentElement.style.setProperty('--primary-foreground', color.primaryForeground);
  }

  function applyZoom(value: number) {
    document.documentElement.style.setProperty('zoom', `${value}%`);
  }

  $effect(() => {
    applyZoom(zoomLevel);
  });

  // ── 数据设置 ──
  let backupDir = $state('');
  let autoBackup = $state<'off' | 'daily' | 'weekly' | 'monthly'>('off');
  let maxBackups = $state(3);
  let compactBackup = $state(false);
  let appDataDir = $state('');
  let appLogDir = $state('');
  let cacheSize = $state('...');
  let clearingCache = $state(false);
  let resettingData = $state(false);
  let backingUp = $state(false);

  // ── 关于我们 ──
  const releaseUrl = 'https://github.com/oooyuy92/orion-chat-rs/releases/latest';
  let appVersion = $state('');
  let autoUpdate = $state(true);
  let checkingUpdate = $state(false);
  let updatePhase = $state('idle');
  let updateVersion = $state('');
  let updateError = $state('');
  let updateDownloadedBytes = $state(0);
  let updateTotalBytes = $state<number | null>(null);
  let updaterController = $state<any>(null);

  function applyUpdateSnapshot(snapshot: any) {
    updatePhase = snapshot.phase ?? 'idle';
    updateVersion = snapshot.version ?? '';
    updateError = snapshot.error ?? '';
    updateDownloadedBytes = snapshot.downloadedBytes ?? 0;
    updateTotalBytes = snapshot.totalBytes ?? null;
  }

  async function ensureUpdaterController() {
    if (!updaterController) {
      updaterController = await createDefaultAutoUpdaterController({ releaseUrl });
    }
    return updaterController;
  }

  async function handleCheckUpdate() {
    checkingUpdate = true;
    updateError = '';
    try {
      const controller = await ensureUpdaterController();
      const result = await controller.checkForUpdates({
        autoDownload: autoUpdate,
        onProgress: (_event: any, snapshot: any) => applyUpdateSnapshot(snapshot),
      });
      applyUpdateSnapshot(result);
    } catch (e) {
      updatePhase = 'error';
      updateError = String(e);
    } finally {
      checkingUpdate = false;
    }
  }

  async function handleDownloadUpdate() {
    try {
      const controller = await ensureUpdaterController();
      const result = await controller.downloadPendingUpdate({
        onProgress: (_event: any, snapshot: any) => applyUpdateSnapshot(snapshot),
      });
      applyUpdateSnapshot(result);
    } catch (e) {
      updatePhase = 'error';
      updateError = String(e);
    }
  }

  async function handleInstallUpdate() {
    const confirmed = window.confirm(language === 'zh' ? '更新已下载，是否立即重启并安装？' : 'The update has been downloaded. Restart now to install it?');
    if (!confirmed) return;

    try {
      const controller = await ensureUpdaterController();
      const result = await controller.installAndRestart();
      applyUpdateSnapshot(result);
    } catch (e) {
      updatePhase = 'error';
      updateError = String(e);
    }
  }

  function openLatestRelease() {
    window.open(releaseUrl, '_blank', 'noopener,noreferrer');
  }

  function formatUpdateBytes(bytes: number | null | undefined) {
    if (bytes == null || Number.isNaN(bytes)) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  async function handlePickBackupDir() {
    try {
      const dir = await api.pickDirectory();
      if (dir) backupDir = dir;
    } catch (e) { console.error(e); }
  }

  async function handleClearBackupDir() {
    backupDir = '';
  }

  async function handleOpenDataDir() {
    if (appDataDir) await api.openPath(appDataDir);
  }

  async function handleOpenLogDir() {
    if (appLogDir) await api.openPath(appLogDir);
  }

  async function handleClearCache() {
    clearingCache = true;
    try {
      await api.clearCache();
      cacheSize = await api.getCacheSize();
    } catch (e) { console.error(e); } finally {
      clearingCache = false;
    }
  }

  async function handleLocalBackup() {
    if (!backupDir) {
      const dir = await api.pickDirectory();
      if (!dir) return;
      backupDir = dir;
    }
    backingUp = true;
    try {
      const now = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      const dest = `${backupDir}/orion-backup-${now}.db`;
      await api.localBackup(dest);
      success = language === 'zh' ? `备份成功：${dest}` : `Backup saved: ${dest}`;
    } catch (e) {
      error = String(e);
    } finally {
      backingUp = false;
    }
  }

  async function handleResetData() {
    const msg = language === 'zh'
      ? '确认重置所有数据？此操作将删除全部对话记录，且不可撤销。'
      : 'Reset all data? This will delete all conversations and cannot be undone.';
    if (!window.confirm(msg)) return;
    resettingData = true;
    try {
      await api.resetAppData();
      success = language === 'zh' ? '数据已重置。' : 'Data has been reset.';
    } catch (e) {
      error = String(e);
    } finally {
      resettingData = false;
    }
  }

  // ── Settings persistence with tauri-plugin-store ──
  let settingsLoaded = $state(false);

  async function loadSettings() {
    try {
      await i18n.init();
      const store = await loadStore('settings.json');
      proxyMode = (await store.get<'system' | 'none'>('proxyMode')) ?? 'system';
      autoBackup = (await store.get<typeof autoBackup>('autoBackup')) ?? 'off';
      maxBackups = (await store.get<number>('maxBackups')) ?? 3;
      compactBackup = (await store.get<boolean>('compactBackup')) ?? false;
      backupDir = (await store.get<string>('backupDir')) ?? '';
      selectedColorIndex = (await store.get<number>('colorIndex')) ?? 0;
      const savedZoom = (await store.get<number>('zoom')) ?? 100;
      autoUpdate = (await store.get<boolean>('autoUpdate')) ?? true;
      autoRename = (await store.get<boolean>('autoRename')) ?? false;
      autoRenameModelId = (await store.get<string>('autoRenameModelId')) ?? '';
      autoCompress = (await store.get<boolean>('autoCompress')) ?? false;
      autoCompressModelId = (await store.get<string>('autoCompressModelId')) ?? '';
      autoCompressThreshold = (await store.get<number>('autoCompressThreshold')) ?? 50000;
      zoomLevel = savedZoom;

      // Apply loaded display settings immediately
      applyThemeColor(selectedColorIndex);
      applyZoom(savedZoom);

      // Load autostart state from backend
      autoLaunch = await api.getAutostartEnabled();
      // Apply proxy from backend
      proxyMode = (await api.getProxyMode()) as 'system' | 'none';
      settingsLoaded = true;
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  }

  async function saveSettings() {
    try {
      const store = await loadStore('settings.json');
      await store.set('proxyMode', proxyMode);
      await store.set('autoBackup', autoBackup);
      await store.set('maxBackups', maxBackups);
      await store.set('compactBackup', compactBackup);
      await store.set('backupDir', backupDir);
      await store.set('colorIndex', selectedColorIndex);
      await store.set('zoom', zoomLevel);
      await store.set('autoUpdate', autoUpdate);
      await store.set('autoRename', autoRename);
      await store.set('autoRenameModelId', autoRenameModelId);
      await store.set('autoCompress', autoCompress);
      await store.set('autoCompressModelId', autoCompressModelId);
      await store.set('autoCompressThreshold', autoCompressThreshold);
      await store.save();
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  }

  // Auto-save display/general settings whenever they change
  $effect(() => {
    void proxyMode; void autoBackup; void maxBackups;
    void compactBackup; void backupDir; void selectedColorIndex; void zoomLevel;
    void autoUpdate; void autoRename; void autoRenameModelId;
    void autoCompress; void autoCompressModelId; void autoCompressThreshold;
    if (!settingsLoaded) return;
    saveSettings();
  });

  const updateStatusText = $derived.by(() => {
    switch (updatePhase) {
      case 'up-to-date':
        return t.upToDate;
      case 'available':
        return updateVersion ? (t.updateAvailable?.(updateVersion) ?? '') : '';
      case 'downloading': {
        const progress = updateTotalBytes && updateTotalBytes > 0
          ? `${formatUpdateBytes(updateDownloadedBytes)} / ${formatUpdateBytes(updateTotalBytes)}`
          : formatUpdateBytes(updateDownloadedBytes);
        return updateVersion ? (t.downloadingUpdate?.(updateVersion, progress) ?? '') : '';
      }
      case 'downloaded':
        return updateVersion ? (t.downloadedUpdate?.(updateVersion) ?? '') : '';
      case 'installing':
        return t.installingUpdate;
      case 'error':
        return updateError ? (t.updateFailed?.(updateError) ?? updateError) : (t.updateFailed?.('Unknown error') ?? 'Unknown error');
      default:
        return '';
    }
  });

  onMount(() => {
    void i18n.init();
    loadProviders();
    loadSettings().then(async () => {
      try {
        const paths = await api.getAppPaths();
        appDataDir = paths.dataDir;
        appLogDir = paths.logDir;
        cacheSize = await api.getCacheSize();
        appVersion = await getVersion();
      } catch (e) { console.error(e); }
    });
  });

  const t = $derived.by(() => {
    if (language === 'en') {
      return {
        generalSettings: 'General Settings',
        language: 'Language',
        proxyMode: 'Proxy Mode',
        proxySystem: 'System Proxy',
        proxyNone: 'No Proxy',
        autoLaunch: 'Launch at Startup',
        modelService: 'Model Service',
        searchProvider: 'Search provider...',
        loading: 'Loading...',
        noMatch: 'No matching providers',
        addBtn: 'Add',
        collapse: 'Collapse',
        providerName: 'Provider Name',
        providerNamePlaceholder: 'e.g. OpenAI Official',
        providerType: 'Provider Type',
        apiAddress: 'API Address',
        apiKey: 'API Key',
        apiKeyOptional: 'Optional (leave empty for Ollama)',
        enableAfterCreate: 'Enable after creation',
        confirmAdd: 'Confirm',
        adding: 'Adding...',
        enabled: 'Enabled',
        disabled: 'Disabled',
        clickToEditName: 'Click to edit name',
        apiKeyPlaceholder: 'Enter API Key',
        show: 'Show',
        hide: 'Hide',
        check: 'Check',
        checking: 'Checking...',
        apiKeyHint: 'Multiple keys can be separated by commas for auto-rotation.',
        models: 'Models',
        modelsCount: (n: number) => `${n} models`,
        searchModel: 'Search models...',
        selectAll: 'Select All',
        deselectAll: 'Deselect All',
        syncModels: 'Sync Models',
        addModel: 'Add Model',
        syncing: 'Syncing...',
        noModels: 'No models. Click "Sync Models" to fetch.',
        noMatchModels: 'No matching models',
        disabledHint: 'Provider is disabled. Model selection is preserved but won\'t show in chat.',
        requestModelName: 'Request Model Name',
        requestModelPlaceholder: 'e.g. gpt-4.1',
        displayModelName: 'Display Name / Remark',
        displayModelPlaceholder: 'Optional label shown in the UI',
        saveModel: 'Save Model',
        savingModel: 'Saving...',
        manualSource: 'Manual',
        addModelTitle: 'Add Manual Model',
        selectProvider: 'Select a Provider',
        selectProviderHint: 'Choose from the list or click "Add" to create one.',
        setDefault: 'Set as Default',
        delete: 'Delete',
        featureWip: 'Feature in development',
        default: 'Default',
        displaySettings: 'Display Settings',
        themeColor: 'Theme Color',
        zoom: 'Zoom',
        zoomValue: (v: number) => `${v}%`,
        autoRename: 'Auto Smart Rename',
        autoRenameModel: 'Rename Model',
        autoRenameHint: 'Renames after the 1st round, then every 10 rounds.',
        autoCompress: 'Auto Context Compression',
        autoCompressModel: 'Compression Model',
        autoCompressThreshold: 'Token Threshold',
        autoCompressHint: 'When context tokens exceed the threshold, automatically summarize and compress.',
        dataSettings: 'Data Settings',
        backupSection: 'Data Backup',
        backupDir: 'Backup Directory',
        browse: 'Browse',
        clear: 'Clear',
        backupRestore: 'Backup & Restore',
        localBackup: 'Local Backup',
        backupManager: 'Backup Files',
        autoBackup: 'Auto Backup',
        autoBackupOff: 'Off',
        autoBackupDaily: 'Daily',
        autoBackupWeekly: 'Weekly',
        autoBackupMonthly: 'Monthly',
        maxBackups: 'Max Backups',
        compactBackup: 'Compact Backup',
        compactBackupHint: 'Only back up settings and conversation records, excluding files and images.',
        dataDir: 'Data Directory',
        appData: 'App Data',
        openDir: 'Open',
        appLog: 'App Logs',
        openLog: 'Open',
        clearCache: 'Clear Cache',
        resetData: 'Reset Data',
        resetDataHint: 'This will delete all local data and cannot be undone.',
        about: 'About',
        tagline: 'The smartest, lowest-friction AI chat.',
        version: 'Version',
        autoUpdate: 'Auto Update',
        checkUpdate: 'Check for Updates',
        checkingUpdate: 'Checking...',
        upToDate: 'You\'re up to date',
      };
    }
    return {
      generalSettings: '常规设置',
      language: '语言',
      proxyMode: '代理模式',
      proxySystem: '系统代理',
      proxyNone: '不使用代理',
      autoLaunch: '开机自启动',
      modelService: '模型服务',
      searchProvider: '搜索模型平台...',
      loading: '正在加载服务...',
      noMatch: '未找到匹配服务',
      addBtn: '+ 添加',
      collapse: '收起',
      providerName: '服务名称',
      providerNamePlaceholder: '例如：OpenAI 官方',
      providerType: '提供商类型',
      apiAddress: 'API 地址',
      apiKey: 'API 密钥',
      apiKeyOptional: '可选（Ollama 可留空）',
      enableAfterCreate: '创建后立即启用',
      confirmAdd: '确认添加',
      adding: '添加中...',
      enabled: '已启用',
      disabled: '已停用',
      clickToEditName: '点击修改名称',
      apiKeyPlaceholder: '输入 API 密钥',
      show: '显示',
      hide: '隐藏',
      check: '检测',
      checking: '检测中...',
      apiKeyHint: '多个密钥可用英文逗号分隔，将自动轮询。',
      models: '模型',
      modelsCount: (n: number) => `${n}个模型`,
      searchModel: '搜索模型...',
      selectAll: '全选',
      deselectAll: '全不选',
      syncModels: '同步模型',
      addModel: '添加模型',
      syncing: '同步中...',
      noModels: '暂无模型，点击"同步模型"拉取。',
      noMatchModels: '无匹配模型',
      disabledHint: '当前服务已停用，模型勾选状态会保留但不会在聊天中显示。',
      requestModelName: '请求模型名',
      requestModelPlaceholder: '例如：gpt-4.1',
      displayModelName: '显示名 / 备注',
      displayModelPlaceholder: '选填，用于界面展示',
      saveModel: '保存模型',
      savingModel: '保存中...',
      manualSource: '手动',
      addModelTitle: '添加手动模型',
      selectProvider: '请选择一个服务',
      selectProviderHint: '你可以在中间列表中选择，或先点击 "+ 添加" 创建新服务。',
      setDefault: '设成默认',
      delete: '删除',
      featureWip: '功能待开发',
      default: '默认',
      displaySettings: '显示设置',
      themeColor: '主题颜色',
      zoom: '界面缩放',
      zoomValue: (v: number) => `${v}%`,
      autoRename: '自动智能重命名',
      autoRenameModel: '重命名模型',
      autoRenameHint: '第1轮对话后重命名一次，此后每10轮更新一次。',
      autoCompress: '自动上下文压缩',
      autoCompressModel: '压缩模型',
      autoCompressThreshold: '压缩阈值（Tokens）',
      autoCompressHint: '当上下文 Token 数超过阈值时，自动总结并压缩对话。',
      dataSettings: '数据设置',
      backupSection: '数据备份',
      backupDir: '备份目录',
      browse: '浏览',
      clear: '清除',
      backupRestore: '数据备份与恢复',
      localBackup: '本地备份',
      backupManager: '备份文件管理',
      autoBackup: '自动备份',
      autoBackupOff: '关闭',
      autoBackupDaily: '每天',
      autoBackupWeekly: '每周',
      autoBackupMonthly: '每月',
      maxBackups: '最大备份数',
      compactBackup: '精简备份',
      compactBackupHint: '仅备份设置和对话记录，不包含文件和图片等附件内容。',
      dataDir: '数据目录',
      appData: '应用数据',
      openDir: '打开目录',
      appLog: '应用日志',
      openLog: '打开日志',
      clearCache: '清除缓存',
      resetData: '重置数据',
      resetDataHint: '将删除所有本地数据，此操作不可撤销。',
      about: '关于',
      tagline: '最智能、最低负担的 AI 聊天',
      version: '版本',
      autoUpdate: '自动更新',
      checkUpdate: '检查更新',
      checkingUpdate: '检查中...',
      upToDate: '已是最新版本',
      updateAvailable: (version: string) => `发现新版本：${version}`,
      downloadUpdate: '下载更新',
      downloadingUpdate: (version: string, progress: string) => `正在下载 ${version}${progress ? `（${progress}）` : ''}` ,
      downloadedUpdate: (version: string) => `更新 ${version} 已下载，重启后安装。`,
      restartToInstall: '重启安装',
      installingUpdate: '正在安装更新...',
      updateFailed: (message: string) => `更新失败：${message}`,
      openReleasePage: '打开发布页',
    };
  });

  let showAddForm = $state(false);
  let creating = $state(false);
  let newName = $state('');
  let newType = $state<ProviderType>('openaiCompat');
  let newApiBase = $state('https://api.openai.com/v1');
  let newApiKey = $state('');
  let newEnabled = $state(true);

  let selectedProvider = $derived.by(() =>
    providers.find((provider) => provider.id === selectedProviderId) ?? null,
  );

  let filteredProviders = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) {
      return providers;
    }
    return providers.filter((provider) => provider.name.toLowerCase().includes(q));
  });

  let isDirty = $derived.by(() => {
    if (!selectedProvider) {
      return false;
    }

    return (
      draftName.trim() !== selectedProvider.name ||
      draftType !== selectedProvider.providerType ||
      draftApiBase.trim() !== selectedProvider.apiBase ||
      draftApiKey !== (selectedProvider.apiKey ?? '') ||
      draftEnabled !== selectedProvider.enabled
    );
  });

  let filteredModels = $derived.by(() => {
    if (!selectedProvider) return [];
    const q = modelSearch.trim().toLowerCase();
    if (!q) return selectedProvider.models;
    return selectedProvider.models.filter((m) =>
      [
        resolveModelLabel(m),
        resolveModelSecondaryLabel(m),
        m.requestName,
        m.id,
      ]
        .join(' ')
        .toLowerCase()
        .includes(q),
    );
  });

  let endpointPreview = $derived.by(() => {
    const base = draftApiBase.trim().replace(/\/+$/, '');
    if (!base) {
      return '';
    }

    switch (draftType) {
      case 'anthropic':
        return `${base}/v1/messages`;
      case 'gemini':
        return `${base}/v1beta/models`;
      case 'ollama':
        return `${base}/api/tags`;
      default:
        return `${base}/chat/completions`;
    }
  });

  function providerTypeDefaultBase(type: ProviderType): string {
    return (
      providerTypeOptions.find((option) => option.value === type)?.defaultBase ??
      'https://api.openai.com/v1'
    );
  }

  function applyDraft(provider: ProviderConfig) {
    draftName = provider.name;
    draftType = provider.providerType;
    draftApiBase = provider.apiBase;
    draftApiKey = provider.apiKey ?? '';
    draftEnabled = provider.enabled;
    showApiKey = false;
  }

  function selectProvider(id: string) {
    selectedProviderId = id;
    modelSearch = '';
    const provider = providers.find((item) => item.id === id);
    if (provider) {
      applyDraft(provider);
    }
  }

  function normalizeProviders(items: ProviderConfig[]): ProviderConfig[] {
    return [...items];
  }

  async function loadProviders() {
    loading = true;
    error = '';
    try {
      providers = normalizeProviders(await api.listProviders());

      if (providers.length > 0) {
        const initial = providers.some((provider) => provider.id === selectedProviderId)
          ? selectedProviderId
          : providers[0].id;
        selectProvider(initial);
      } else {
        selectedProviderId = '';
      }
    } catch (e) {
      console.error('Failed to load providers:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleSyncModels(providerId = selectedProviderId) {
    if (!providerId) {
      return;
    }

    syncingModels = { ...syncingModels, [providerId]: true };
    success = '';
    error = '';

    try {
      const models = await api.fetchModels(providerId);
      providers = providers.map((provider) =>
        provider.id === providerId ? { ...provider, models } : provider,
      );
      success = language === 'zh' ? `模型同步成功，共 ${models.length} 个模型。` : `Model sync succeeded with ${models.length} models.`;
    } catch (e) {
      console.error('Failed to fetch models:', e);
      error = language === 'zh' ? `模型检测失败：${e}` : `Model check failed: ${e}`;
    } finally {
      syncingModels = { ...syncingModels, [providerId]: false };
    }
  }

  async function handleToggleModelVisibility(modelId: string, enabled: boolean) {
    if (!selectedProvider || !draftEnabled) {
      return;
    }

    updatingModels = { ...updatingModels, [modelId]: true };
    error = '';
    success = '';

    try {
      await api.updateModelVisibility(modelId, enabled);
      providers = providers.map((provider) =>
        provider.id === selectedProvider.id
          ? {
              ...provider,
              models: provider.models.map((model) =>
                model.id === modelId ? { ...model, enabled } : model,
              ),
            }
          : provider,
      );
    } catch (e) {
      console.error('Failed to update model visibility:', e);
      error = language === 'zh' ? `更新模型显示状态失败：${e}` : `Failed to update model visibility: ${e}`;
    } finally {
      updatingModels = { ...updatingModels, [modelId]: false };
    }
  }

  async function handleBatchModelVisibility(enabled: boolean) {
    if (!selectedProvider || !draftEnabled || selectedProvider.models.length === 0) {
      return;
    }

    bulkUpdatingModels = true;
    error = '';
    success = '';

    try {
      await api.updateProviderModelsVisibility(selectedProvider.id, enabled);
      providers = providers.map((provider) =>
        provider.id === selectedProvider.id
          ? {
              ...provider,
              models: provider.models.map((model) => ({ ...model, enabled })),
            }
          : provider,
      );
      success = enabled
        ? (language === 'zh' ? '已勾选该服务下全部模型。' : 'All models under this provider are now selected.')
        : (language === 'zh' ? '已取消该服务下全部模型。' : 'All models under this provider are now deselected.');
    } catch (e) {
      console.error('Failed to batch update model visibility:', e);
      error = language === 'zh' ? `批量更新模型显示状态失败：${e}` : `Failed to update model visibility in bulk: ${e}`;
    } finally {
      bulkUpdatingModels = false;
    }
  }

  async function handleSaveSelected() {
    if (!selectedProvider) {
      return;
    }

    const nextName = draftName.trim();
    const nextBase = draftApiBase.trim();

    if (!nextName || !nextBase) {
      error = language === 'zh' ? '服务名称和 API 地址不能为空。' : 'Provider name and API address are required.';
      return;
    }

    saving = true;
    error = '';
    success = '';

    try {
      const updated = await api.updateProvider(
        selectedProvider.id,
        nextName,
        draftType,
        nextBase,
        draftApiKey.trim() || null,
        draftEnabled,
      );

      providers = normalizeProviders(
        providers.map((provider) => (provider.id === updated.id ? updated : provider)),
      );
      selectProvider(updated.id);
    } catch (e) {
      console.error('Failed to update provider:', e);
      error = language === 'zh' ? `保存失败：${e}` : `Save failed: ${e}`;
    } finally {
      saving = false;
    }
  }

  async function handleCreateProvider() {
    const name = newName.trim();
    const base = newApiBase.trim();

    if (!name || !base) {
      error = language === 'zh' ? '请先填写服务名称和 API 地址。' : 'Please enter a provider name and API address first.';
      return;
    }

    creating = true;
    error = '';
    success = '';

    try {
      const provider = await api.addProvider(
        name,
        newType,
        base,
        newApiKey.trim() || undefined,
        newEnabled,
      );

      providers = normalizeProviders([...providers, provider]);
      selectProvider(provider.id);

      showAddForm = false;
      newName = '';
      newType = 'openaiCompat';
      newApiBase = providerTypeDefaultBase('openaiCompat');
      newApiKey = '';
      newEnabled = true;
      success = language === 'zh' ? '服务已添加。' : 'Provider added.';
    } catch (e) {
      console.error('Failed to create provider:', e);
      error = language === 'zh' ? `添加失败：${e}` : `Add failed: ${e}`;
    } finally {
      creating = false;
    }
  }

  function handleDraftTypeChange(next: ProviderType) {
    draftType = next;
    if (!draftApiBase.trim()) {
      draftApiBase = providerTypeDefaultBase(next);
    }
  }

  function handleNewTypeChange(next: ProviderType) {
    newType = next;
    if (!newApiBase.trim()) {
      newApiBase = providerTypeDefaultBase(next);
    }
  }

  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function resetManualModelDraft() {
    manualModelRequestName = '';
    manualModelDisplayName = '';
    manualModelEnabled = true;
  }

  $effect(() => {
    if (!showAddModelDialog) {
      resetManualModelDraft();
    }
  });

  async function handleCreateManualModel() {
    if (!selectedProvider) return;
    const providerId = selectedProvider.id;
    const requestName = manualModelRequestName.trim();
    if (!requestName) {
      error = language === 'zh' ? '请求模型名不能为空。' : 'Request model name is required.';
      return;
    }

    creatingManualModel = true;
    error = '';
    success = '';

    try {
      const model = await api.createManualModel(
        providerId,
        requestName,
        manualModelDisplayName.trim() || null,
        manualModelEnabled,
      );

      providers = providers.map((provider) =>
        provider.id === providerId
          ? { ...provider, models: [...provider.models, model] }
          : provider,
      );
      showAddModelDialog = false;
      success = language === 'zh' ? '模型已添加。' : 'Model added.';
    } catch (e) {
      console.error('Failed to create manual model:', e);
      error = language === 'zh' ? `添加模型失败：${e}` : `Failed to add model: ${e}`;
    } finally {
      creatingManualModel = false;
    }
  }

  $effect(() => {
    if (!isDirty) return;

    // Access reactive deps
    void draftName;
    void draftType;
    void draftApiBase;
    void draftApiKey;
    void draftEnabled;

    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      handleSaveSelected();
    }, 600);

    return () => {
      if (autoSaveTimer) clearTimeout(autoSaveTimer);
    };
  });

  function handleContextMenu(e: MouseEvent, providerId: string) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, providerId };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  async function handleContextDelete() {
    if (!contextMenu) return;
    const target = providers.find((p) => p.id === contextMenu!.providerId);
    if (!target) return;

    const confirmed = window.confirm(language === 'zh' ? `确认删除服务 "${target.name}" 吗？` : `Delete provider "${target.name}"?`);
    closeContextMenu();
    if (!confirmed) return;

    deleting = true;
    error = '';
    success = '';

    try {
      await api.deleteProvider(target.id);
      providers = providers.filter((p) => p.id !== target.id);

      if (providers.length > 0) {
        if (selectedProviderId === target.id) {
          selectProvider(providers[0].id);
        }
      } else {
        selectedProviderId = '';
        draftName = '';
        draftApiBase = '';
        draftApiKey = '';
      }

      success = language === 'zh' ? '服务已删除。' : 'Provider deleted.';
    } catch (e) {
      console.error('Failed to delete provider:', e);
      error = language === 'zh' ? `删除失败：${e}` : `Delete failed: ${e}`;
    } finally {
      deleting = false;
    }
  }

  function handleSetDefault() {
    if (!contextMenu) return;
    const targetId = contextMenu.providerId;
    closeContextMenu();

    const idx = providers.findIndex((p) => p.id === targetId);
    if (idx <= 0) return; // already first or not found

    const updated = [...providers];
    const [moved] = updated.splice(idx, 1);
    updated.unshift(moved);
    providers = updated;
    selectProvider(targetId);
  }

  async function startEditName() {
    editingName = true;
    await tick();
    editNameInput?.focus();
    editNameInput?.select();
  }

  function finishEditName() {
    editingName = false;
  }

  function handleNameKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      editingName = false;
    }
  }
</script>

<div class="settings-root">
  <!-- Left Nav -->
  <aside class="settings-nav">
    {#each sectionGroups as group}
      <section class="nav-group">
        <h3>{group.title}</h3>
        {#each group.items as item}
          <button class="nav-item" class:is-active={item === activeNav} onclick={() => activeNav = item}>{navLabel(item)}</button>
        {/each}
      </section>
    {/each}
  </aside>

  {#if activeNav === 'modelService'}
    <!-- Provider List -->
    <section class="provider-list-panel">
      <div class="panel-head">
        <h2>{t.modelService}</h2>
        <Input type="search" bind:value={search} placeholder={t.searchProvider} />
      </div>

    <div class="provider-list-scroll">
      {#if loading}
        <p class="panel-status">{t.loading}</p>
      {:else if filteredProviders.length === 0}
        <p class="panel-status">{t.noMatch}</p>
      {:else}
        {#each filteredProviders as provider, i (provider.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <button
            type="button"
            class="provider-card"
            class:is-active={provider.id === selectedProviderId}
            onclick={() => selectProvider(provider.id)}
            oncontextmenu={(e) => handleContextMenu(e, provider.id)}
          >
            <span class="provider-name">{provider.name}</span>
            <span class="provider-card-badges">
              {#if i === 0}
                <span class="provider-badge">{t.default}</span>
              {/if}
              <span class="provider-state" class:enabled={provider.enabled}>
                {provider.enabled ? (language === 'zh' ? '开启' : 'ON') : (language === 'zh' ? '关闭' : 'OFF')}
              </span>
            </span>
          </button>
        {/each}
      {/if}
    </div>

    <div class="add-provider">
      <Button variant="outline" class="w-full" onclick={() => (showAddForm = !showAddForm)}>
        {showAddForm ? t.collapse : t.addBtn}
      </Button>

      {#if showAddForm}
        <div class="add-form">
          <label>
            {t.providerName}
            <Input bind:value={newName} placeholder={t.providerNamePlaceholder} />
          </label>

          <label>
            {t.providerType}
            <select
              class="form-input"
              value={newType}
              onchange={(event) => handleNewTypeChange((event.target as HTMLSelectElement).value as ProviderType)}
            >
              {#each providerTypeOptions as option (option.value)}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>

          <label>
            {t.apiAddress}
            <Input bind:value={newApiBase} placeholder="https://api.example.com/v1" />
          </label>

          <label>
            {t.apiKey}
            <Input bind:value={newApiKey} type="password" placeholder={t.apiKeyOptional} />
          </label>

          <label class="switch-row">
            <input type="checkbox" checked={newEnabled} onchange={(e) => newEnabled = (e.target as HTMLInputElement).checked} />
            <span>{t.enableAfterCreate}</span>
          </label>

          <Button onclick={handleCreateProvider} disabled={creating} class="w-full">
            {creating ? t.adding : t.confirmAdd}
          </Button>
        </div>
      {/if}
    </div>
  </section>

  <!-- Detail Panel -->
  <section class="detail-panel">
    {#if error}
      <div class="alert alert-error">{error}</div>
    {/if}
    {#if success}
      <div class="alert alert-success">{success}</div>
    {/if}

    {#if selectedProvider}
      <!-- Header -->
      <div class="detail-header">
        {#if editingName}
          <input
            bind:this={editNameInput}
            class="editable-name-input"
            bind:value={draftName}
            onblur={finishEditName}
            onkeydown={handleNameKeydown}
          />
        {:else}
          <button type="button" class="editable-name" onclick={startEditName} title={t.clickToEditName}>{draftName || selectedProvider.name}</button>
        {/if}
        <label class="toggle-label">
          <span class="toggle-text">{draftEnabled ? t.enabled : t.disabled}</span>
          <button type="button" class="toggle-switch" class:is-on={draftEnabled} aria-pressed={draftEnabled} aria-label={draftEnabled ? t.enabled : t.disabled} onclick={() => draftEnabled = !draftEnabled}>
            <span class="toggle-thumb"></span>
          </button>
        </label>
      </div>

      <!-- Provider Type -->
      <div class="detail-section">
        <label class="field-label">
          {t.providerType}
          <select
            class="form-input"
            value={draftType}
            onchange={(event) => handleDraftTypeChange((event.target as HTMLSelectElement).value as ProviderType)}
          >
            {#each providerTypeOptions as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </label>
      </div>

      <!-- API Key -->
      <div class="detail-section">
        <span class="field-label">{t.apiKey}</span>
        <div class="key-input-row">
          <Input
            bind:value={draftApiKey}
            type={showApiKey ? 'text' : 'password'}
            placeholder={t.apiKeyPlaceholder}
          />
          <Button variant="outline" size="sm" onclick={() => (showApiKey = !showApiKey)}>
            {showApiKey ? t.hide : t.show}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={syncingModels[selectedProvider.id] || !draftEnabled}
            onclick={() => handleSyncModels(selectedProvider.id)}
          >
            {syncingModels[selectedProvider.id] ? t.checking : t.check}
          </Button>
        </div>
        <p class="field-hint">{t.apiKeyHint}</p>
      </div>

      <!-- API Address -->
      <div class="detail-section">
        <label class="field-label">
          {t.apiAddress}
          <Input bind:value={draftApiBase} placeholder="https://api.example.com/v1" />
        </label>
        {#if endpointPreview}
          <p class="field-hint">{draftName || selectedProvider.name} → {endpointPreview}</p>
        {/if}
      </div>

      <!-- Models -->
      <div class="detail-section">
        <div class="models-header">
          <div class="models-title">
            <span>{t.models}</span>
            {#if selectedProvider.models.length > 0}
              <span class="model-count">{t.modelsCount(selectedProvider.models.length)}</span>
            {/if}
          </div>
          <div class="models-actions">
            <Input type="search" bind:value={modelSearch} placeholder={t.searchModel} class="model-search" />
            <Button
              variant="outline"
              size="sm"
              onclick={() => handleBatchModelVisibility(true)}
              disabled={!draftEnabled || bulkUpdatingModels || selectedProvider.models.length === 0}
            >
              {t.selectAll}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onclick={() => handleBatchModelVisibility(false)}
              disabled={!draftEnabled || bulkUpdatingModels || selectedProvider.models.length === 0}
            >
              {t.deselectAll}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onclick={() => handleSyncModels(selectedProvider.id)}
              disabled={syncingModels[selectedProvider.id] || !draftEnabled}
            >
              {syncingModels[selectedProvider.id] ? t.syncing : t.syncModels}
            </Button>
            <Button
              variant="outline"
              size="sm"
              class="manualModelAddBtn"
              onclick={() => (showAddModelDialog = true)}
              disabled={!selectedProvider}
            >
              {t.addModel}
            </Button>
          </div>
        </div>

        {#if selectedProvider.models.length > 0}
          <div class="model-grid-scroll">
            <div class="model-grid">
              {#each filteredModels as model (model.id)}
                <button
                  class="model-card"
                  class:is-enabled={model.enabled}
                  disabled={!draftEnabled || updatingModels[model.id] || bulkUpdatingModels}
                  onclick={() => handleToggleModelVisibility(model.id, !model.enabled)}
                >
                  <span class="model-card-primary">{resolveModelLabel(model)}</span>
                  {#if resolveModelSecondaryLabel(model)}
                    <span class="model-card-secondary">{resolveModelSecondaryLabel(model)}</span>
                  {/if}
                  {#if isManualModel(model)}
                    <span class="model-source-badge">{t.manualSource}</span>
                  {/if}
                </button>
              {/each}
            </div>
            {#if filteredModels.length === 0}
              <p class="panel-status">{t.noMatchModels}</p>
            {/if}
          </div>
        {:else}
          <p class="panel-status">{t.noModels}</p>
        {/if}

        {#if !draftEnabled}
          <p class="panel-status" style="margin-top: 0.35rem;">{t.disabledHint}</p>
        {/if}
      </div>

    {:else}
      <div class="empty-state">
        <h3>{t.selectProvider}</h3>
        <p>{t.selectProviderHint}</p>
      </div>
    {/if}
  </section>
  {:else if activeNav === 'assistants'}
    <section class="assistants-panel">
      <AssistantSettings />
    </section>
  {:else if activeNav === 'modelCombos'}
    {#snippet comboEditor()}
      <div class="combo-editor">
        <div class="combo-field">
          <span class="combo-label">{i18n.t.comboName}</span>
          <Input bind:value={editComboName} placeholder={i18n.t.comboName} />
        </div>

        <div class="combo-field">
          <span class="combo-label">{i18n.t.comboModels}</span>
          <div class="combo-slots-row">
            {#each editComboModelIds as modelId, idx (idx)}
              {@const info = resolveComboModelName(modelId)}
              <div class="combo-slot filled" class:picking={slotPickerIndex === idx}>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="combo-slot-body" onclick={() => openSlotPicker(idx)}>
                  <span class="combo-slot-name" title={info.name}>{info.name}</span>
                  <span class="combo-slot-provider">{info.providerName}</span>
                </div>
                <button class="combo-slot-remove" onclick={(e) => { e.stopPropagation(); removeComboModelAt(idx); }} title={i18n.t.delete}>&times;</button>
              </div>
            {/each}
            <!-- Add slot (dashed) -->
            <button class="combo-slot dashed" class:picking={slotPickerIndex === -1} onclick={() => openSlotPicker(-1)}>
              <span class="combo-slot-plus">+</span>
            </button>
          </div>
        </div>

        {#if slotPickerIndex !== null}
          <div class="combo-picker">
            <span class="combo-picker-title">
              {slotPickerIndex === -1
                ? (language === 'en' ? 'Add Model' : '添加模型')
                : (language === 'en' ? 'Swap Model' : '更换模型')}
            </span>
            <div class="combo-picker-list">
              {#each allEnabledModels as model (model.id)}
                <button class="combo-picker-item" onclick={() => pickModelForSlot(model.id)}>
                  <span class="combo-picker-model-name">{model.name}</span>
                  <span class="combo-picker-model-provider">{model.providerName}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="combo-actions">
          <Button size="sm" variant="outline" onclick={cancelEditCombo}>{i18n.t.cancel}</Button>
          <Button size="sm" onclick={saveCombo} disabled={!editComboName.trim() || editComboModelIds.length < 2}>
            {language === 'en' ? 'Save' : '保存'}
          </Button>
        </div>
      </div>
    {/snippet}

    <section class="combos-panel">
      <div class="detail-header">
        <h2>{i18n.t.modelCombos}</h2>
        <Button size="sm" onclick={startNewCombo}>{i18n.t.addCombo}</Button>
      </div>

      {#if combos.length === 0 && editingComboId !== 'new'}
        <div class="combo-empty-state">
          <p>{i18n.t.noCombo}</p>
        </div>
      {:else}
        <div class="combo-list-settings">
          {#each combos as combo (combo.id)}
            {#if editingComboId === combo.id}
              {@render comboEditor()}
            {:else}
              <div class="combo-row">
                <div class="combo-row-info">
                  <span class="combo-row-name">{combo.name}</span>
                  <span class="combo-row-count">{combo.modelIds.length} models</span>
                </div>
                <div class="combo-row-actions">
                  <Button size="sm" variant="outline" onclick={() => startEditCombo(combo)}>
                    {i18n.t.edit}
                  </Button>
                  <Button size="sm" variant="outline" onclick={() => handleDeleteCombo(combo.id)}>
                    {i18n.t.delete}
                  </Button>
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {/if}

      {#if editingComboId === 'new'}
        {@render comboEditor()}
      {/if}
    </section>
  {:else if activeNav === 'generalSettings'}
    <section class="general-panel">
      <div class="detail-header">
        <h2>{t.generalSettings}</h2>
      </div>

      <div class="detail-section">
        <label class="field-label">
          {t.language}
          <select
            class="form-input"
            value={language}
            onchange={(e) => i18n.setLanguage((e.target as HTMLSelectElement).value as Language)}
          >
            <option value="zh">{language === 'en' ? 'Chinese' : '中文'}</option>
            <option value="en">{language === 'en' ? 'English' : '英文'}</option>
          </select>
        </label>
      </div>

      <div class="detail-section">
        <label class="field-label">
          {t.proxyMode}
          <select class="form-input" value={proxyMode}
            onchange={(e) => handleProxyChange((e.target as HTMLSelectElement).value as 'system' | 'none')}>
            <option value="system">{t.proxySystem}</option>
            <option value="none">{t.proxyNone}</option>
          </select>
        </label>
      </div>

      <div class="detail-section">
        <div class="general-switch-row">
          <span class="field-label">{t.autoLaunch}</span>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <button type="button" class="toggle-switch" class:is-on={autoLaunch} aria-pressed={autoLaunch} aria-label={t.autoLaunch} onclick={() => handleAutoLaunchChange(!autoLaunch)}>
            <span class="toggle-thumb"></span>
          </button>
        </div>
      </div>
    </section>
  {:else if activeNav === 'displaySettings'}
    <section class="general-panel">
      <div class="detail-header">
        <h2>{t.displaySettings}</h2>
      </div>

      <div class="detail-section">
        <span class="field-label">{t.themeColor}</span>
        <div class="color-swatches">
          {#each themeColors as color, i}
            <button
              class="color-swatch"
              class:is-selected={selectedColorIndex === i}
              style="background: {color.primary};"
              title={language === 'en' ? color.nameEn : color.name}
              onclick={() => applyThemeColor(i)}
            >
              {#if selectedColorIndex === i}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
              {/if}
            </button>
          {/each}
        </div>
      </div>

      <div class="detail-section">
        <div class="zoom-header">
          <span class="field-label">{t.zoom}</span>
          <span class="zoom-value">{t.zoomValue(zoomLevel)}</span>
        </div>
        <Slider type="single" bind:value={zoomLevel} min={50} max={150} step={5} class="zoom-slider" />
      </div>

      <div class="detail-section">
        <div class="general-switch-row">
          <span class="field-label">{t.autoRename}</span>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <button type="button" class="toggle-switch" class:is-on={autoRename} aria-pressed={autoRename} aria-label={t.autoRename} onclick={() => { autoRename = !autoRename; }}>
            <span class="toggle-thumb"></span>
          </button>
        </div>
        <p class="field-hint">{t.autoRenameHint}</p>
        {#if autoRename}
          <div style="margin-top: 0.6rem;">
            <label class="field-label">
              {t.autoRenameModel}
              <select class="form-input" bind:value={autoRenameModelId}>
                {#each allModelGroups as group}
                  <optgroup label={group.providerName}>
                    {#each group.models as model}
                      <option value={model.id}>{model.name || model.id}</option>
                    {/each}
                  </optgroup>
                {/each}
              </select>
            </label>
          </div>
        {/if}
      </div>

      <div class="detail-section">
        <div class="general-switch-row">
          <span class="field-label">{t.autoCompress}</span>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <button type="button" class="toggle-switch" class:is-on={autoCompress} aria-pressed={autoCompress} aria-label={t.autoCompress} onclick={() => { autoCompress = !autoCompress; }}>
            <span class="toggle-thumb"></span>
          </button>
        </div>
        <p class="field-hint">{t.autoCompressHint}</p>
        {#if autoCompress}
          <div style="margin-top: 0.6rem; display: flex; flex-direction: column; gap: 0.5rem;">
            <label class="field-label">
              {t.autoCompressModel}
              <select class="form-input" bind:value={autoCompressModelId}>
                {#each allModelGroups as group}
                  <optgroup label={group.providerName}>
                    {#each group.models as model}
                      <option value={model.id}>{model.name || model.id}</option>
                    {/each}
                  </optgroup>
                {/each}
              </select>
            </label>
            <label class="field-label">
              {t.autoCompressThreshold}
              <input class="form-input" type="number" bind:value={autoCompressThreshold} min={1000} step={1000} />
            </label>
          </div>
        {/if}
      </div>
    </section>
  {:else if activeNav === 'dataSettings'}
    <section class="general-panel">
      <div class="detail-header">
        <h2>{t.dataSettings}</h2>
      </div>

      <!-- 数据备份 -->
      <div class="detail-section">
        <span class="section-subtitle">{t.backupSection}</span>

        <div class="data-row">
          <span class="data-row-label">{t.backupDir}</span>
          <div class="data-row-content">
            <span class="data-path">{backupDir || '—'}</span>
            <Button variant="outline" size="sm" onclick={handlePickBackupDir}>{t.browse}</Button>
            <Button variant="outline" size="sm" onclick={handleClearBackupDir}>{t.clear}</Button>
          </div>
        </div>

        <div class="data-row">
          <span class="data-row-label">{t.backupRestore}</span>
          <div class="data-row-content">
            <Button variant="outline" size="sm" onclick={handleLocalBackup} disabled={backingUp}>
              {backingUp ? (language === 'zh' ? '备份中…' : 'Backing up…') : t.localBackup}
            </Button>
            <Button variant="outline" size="sm">{t.backupManager}</Button>
          </div>
        </div>

        <div class="data-row">
          <span class="data-row-label">{t.autoBackup}</span>
          <div class="data-row-content">
            <select class="form-input form-input-sm" bind:value={autoBackup}>
              <option value="off">{t.autoBackupOff}</option>
              <option value="daily">{t.autoBackupDaily}</option>
              <option value="weekly">{t.autoBackupWeekly}</option>
              <option value="monthly">{t.autoBackupMonthly}</option>
            </select>
          </div>
        </div>

        <div class="data-row">
          <span class="data-row-label">{t.maxBackups}</span>
          <div class="data-row-content">
            <Input type="number" bind:value={maxBackups} min={1} max={99} class="input-sm" />
          </div>
        </div>

        <div class="data-row">
          <span class="data-row-label">{t.compactBackup}</span>
          <div class="data-row-content">
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <button type="button" class="toggle-switch" class:is-on={compactBackup} aria-pressed={compactBackup} aria-label={t.compactBackup} onclick={() => compactBackup = !compactBackup}>
              <span class="toggle-thumb"></span>
            </button>
          </div>
        </div>
        <p class="field-hint">{t.compactBackupHint}</p>
      </div>

      <!-- 数据目录 -->
      <div class="detail-section">
        <span class="section-subtitle">{t.dataDir}</span>

        <div class="data-row">
          <span class="data-row-label">{t.clearCache}</span>
          <div class="data-row-content">
            <span class="data-value">{cacheSize}</span>
            <Button variant="outline" size="sm" onclick={handleClearCache} disabled={clearingCache}>
              {clearingCache ? (language === 'zh' ? '清除中…' : 'Clearing…') : t.clearCache}
            </Button>
          </div>
        </div>

        <div class="data-row">
          <span class="data-row-label">{t.resetData}</span>
          <div class="data-row-content">
            <Button variant="destructive" size="sm" onclick={handleResetData} disabled={resettingData}>
              {resettingData ? (language === 'zh' ? '重置中…' : 'Resetting…') : t.resetData}
            </Button>
          </div>
        </div>
        <p class="field-hint">{t.resetDataHint}</p>
      </div>
    </section>
  {:else if activeNav === 'about'}
    <section class="general-panel">
      <div class="detail-header">
        <h2>{t.about}</h2>
      </div>

      <div class="about-hero">
        <div class="about-title-block">
          <span class="about-app-name">Orion Chat</span>
          <span class="about-tagline">{t.tagline}</span>
          <span class="about-version">{t.version} {appVersion || '—'}</span>
        </div>
        <a
          class="github-badge"
          href="https://github.com/oooyuy92/orion-chat-rs"
          target="_blank"
          rel="noopener noreferrer"
          aria-label={language === 'zh' ? 'GitHub 仓库' : 'GitHub repository'}
        >
          <svg height="20" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M12 0C5.37 0 0 5.37 0 12c0 5.3 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61-.546-1.385-1.335-1.755-1.335-1.755-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 21.795 24 17.295 24 12c0-6.63-5.37-12-12-12z"/>
          </svg>
          <span>{language === 'zh' ? 'GitHub 仓库' : 'GitHub'}</span>
        </a>
      </div>

      <div class="detail-section">
        <div class="general-switch-row">
          <span class="field-label">{t.autoUpdate}</span>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <button type="button" class="toggle-switch" class:is-on={autoUpdate} aria-pressed={autoUpdate} aria-label={t.autoUpdate} onclick={() => { autoUpdate = !autoUpdate; }}>
            <span class="toggle-thumb"></span>
          </button>
        </div>
      </div>

      <div class="detail-section">
        <div class="about-update-row">
          <Button variant="outline" size="sm" onclick={handleCheckUpdate} disabled={checkingUpdate || updatePhase === 'downloading' || updatePhase === 'installing'}>
            {checkingUpdate ? t.checkingUpdate : t.checkUpdate}
          </Button>
          {#if updatePhase === 'available'}
            <Button variant="outline" size="sm" onclick={handleDownloadUpdate}>
              {t.downloadUpdate}
            </Button>
          {:else if updatePhase === 'downloaded'}
            <Button size="sm" onclick={handleInstallUpdate}>
              {t.restartToInstall}
            </Button>
          {:else if updatePhase === 'error'}
            <Button variant="outline" size="sm" onclick={openLatestRelease}>
              {t.openReleasePage}
            </Button>
          {/if}

          {#if updateStatusText}
            <span class="update-status" class:is-error={updatePhase === 'error'}>{updateStatusText}</span>
          {/if}
        </div>
      </div>
    </section>
  {:else}
    <section class="placeholder-panel">
      <div class="empty-state">
        <h3>{navLabel(activeNav)}</h3>
        <p>{t.featureWip}</p>
      </div>
    </section>
  {/if}
</div>

<Dialog.Root bind:open={showAddModelDialog}>
  <Dialog.Content class="manual-model-dialog">
    <Dialog.Header>
      <Dialog.Title>{t.addModelTitle}</Dialog.Title>
      <Dialog.Description>
        {selectedProvider ? selectedProvider.name : ''}
      </Dialog.Description>
    </Dialog.Header>

    <div class="manual-model-form">
      <label class="field-label">
        {t.requestModelName}
        <Input
          bind:value={manualModelRequestName}
          placeholder={t.requestModelPlaceholder}
        />
      </label>

      <label class="field-label">
        {t.displayModelName}
        <Input
          bind:value={manualModelDisplayName}
          placeholder={t.displayModelPlaceholder}
        />
      </label>

      <label class="switch-row">
        <input
          type="checkbox"
          checked={manualModelEnabled}
          onchange={(e) => (manualModelEnabled = (e.target as HTMLInputElement).checked)}
        />
        <span>{t.enabled}</span>
      </label>
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showAddModelDialog = false)}>
        {language === 'en' ? 'Cancel' : '取消'}
      </Button>
      <Button onclick={handleCreateManualModel} disabled={creatingManualModel || !manualModelRequestName.trim()}>
        {creatingManualModel ? t.savingModel : t.saveModel}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

{#if contextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="context-overlay" role="button" aria-label={i18n.t.close} tabindex="0" onclick={closeContextMenu} onkeydown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ' ) { e.preventDefault(); closeContextMenu(); } }} oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}>
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px;">
      {#if providers.length > 0 && providers[0].id !== contextMenu.providerId}
        <button class="context-item" onclick={handleSetDefault}>{t.setDefault}</button>
      {/if}
      <button class="context-item danger" onclick={handleContextDelete}>{t.delete}</button>
    </div>
  </div>
{/if}

<style>
  /* ── Root Layout ── */
  .settings-root {
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-columns: 12.5rem 16rem minmax(0, 1fr);
    border-top: 1px solid var(--border);
    background: var(--background);
  }

  /* ── Left Nav ── */
  .settings-nav {
    border-right: 1px solid var(--border);
    background: var(--sidebar);
    padding: 0.9rem 0.65rem;
    overflow-y: auto;
    min-height: 0;
  }

  .nav-group {
    padding-bottom: 0.75rem;
    margin-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .nav-group:last-child {
    margin-bottom: 0;
    border-bottom: none;
  }

  .nav-group h3 {
    margin: 0 0 0.45rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .nav-item {
    width: 100%;
    text-align: left;
    background: transparent;
    color: var(--foreground);
    border: none;
    border-radius: 0.5rem;
    padding: 0.4rem 0.55rem;
    font-size: 0.82rem;
    cursor: pointer;
    transition: background 0.12s;
  }

  .nav-item:hover {
    background: var(--muted);
  }

  .nav-item.is-active {
    background: var(--muted);
    font-weight: 600;
  }

  /* ── Provider List Panel ── */
  .provider-list-panel {
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    background: var(--card);
    min-height: 0;
  }

  .panel-head {
    padding: 0.9rem 0.75rem 0.7rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2 {
    margin: 0 0 0.55rem;
    font-size: 0.9rem;
    font-weight: 650;
  }

  .provider-list-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .provider-card {
    appearance: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 0.5rem;
    padding: 0.5rem 0.6rem;
    font-size: 0.84rem;
    cursor: pointer;
    color: var(--foreground);
    user-select: none;
    transition: background 0.15s, border-color 0.15s;
  }

  .provider-card:hover {
    background: var(--muted);
  }

  .provider-card.is-active {
    background: var(--muted);
    border-color: var(--border);
  }

  .provider-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-state {
    font-size: 0.66rem;
    border-radius: 999px;
    padding: 0.1rem 0.36rem;
    color: #9f1d1d;
    background: #fee2e2;
    border: 1px solid #fecaca;
    flex-shrink: 0;
  }

  .provider-state.enabled {
    color: #166534;
    background: #dcfce7;
    border-color: #bbf7d0;
  }

  .provider-badge {
    font-size: 0.66rem;
    border-radius: 999px;
    padding: 0.1rem 0.36rem;
    color: var(--primary-foreground);
    background: var(--primary);
    flex-shrink: 0;
  }

  .provider-card-badges {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .add-provider {
    border-top: 1px solid var(--border);
    padding: 0.6rem;
  }

  .add-form {
    margin-top: 0.55rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .add-form label {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  .switch-row {
    display: inline-flex;
    align-items: center;
    gap: 0.36rem;
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  /* ── Detail Panel ── */
  .detail-panel {
    padding: 1.2rem 1.5rem;
    overflow-y: auto;
    min-height: 0;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 1.2rem;
    padding-bottom: 0.8rem;
    border-bottom: 1px solid var(--border);
  }

  .detail-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 700;
  }

  /* Editable Name */
  .editable-name {
    margin: 0;
    background: transparent;
    border: none;
    color: var(--foreground);
    text-align: left;
    font-size: 1.1rem;
    font-weight: 700;
    cursor: pointer;
    border-radius: 0.35rem;
    padding: 0.1rem 0.3rem;
    margin: -0.1rem -0.3rem;
    transition: background 0.12s;
  }

  .editable-name:hover {
    background: var(--muted);
  }

  .editable-name-input {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--foreground);
    background: var(--background);
    border: 1px solid var(--primary);
    border-radius: 0.35rem;
    padding: 0.1rem 0.3rem;
    outline: none;
    width: auto;
    min-width: 8rem;
  }

  /* Toggle Switch */
  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    user-select: none;
  }

  .toggle-text {
    font-size: 0.78rem;
    color: var(--muted-foreground);
  }

  .toggle-switch {
    appearance: none;
    padding: 0;
    position: relative;
    width: 2.4rem;
    height: 1.3rem;
    background: var(--muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-switch.is-on {
    background: #22c55e;
    border-color: #22c55e;
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 0.9rem;
    height: 0.9rem;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }

  .toggle-switch.is-on .toggle-thumb {
    transform: translateX(1.1rem);
  }

  /* Detail Sections */
  .detail-section {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .detail-section:last-of-type {
    border-bottom: none;
  }

  .field-label {
    display: flex;
    flex-direction: column;
    gap: 0.32rem;
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--foreground);
  }

  .field-hint {
    margin: 0.3rem 0 0;
    font-size: 0.72rem;
    color: var(--muted-foreground);
  }

  .key-input-row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.35rem;
    align-items: center;
    margin-top: 0.32rem;
  }

  /* Form Input (native select) */
  .form-input {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--background);
    color: var(--foreground);
    padding: 0.5rem 0.6rem;
    font-size: 0.84rem;
    box-sizing: border-box;
    outline: none;
    transition: border-color 0.15s;
  }

  .form-input:focus {
    border-color: var(--ring);
  }

  /* Models Section */
  .models-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.65rem;
  }

  .models-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.88rem;
    font-weight: 600;
  }

  .model-count {
    font-size: 0.72rem;
    font-weight: 400;
    color: var(--muted-foreground);
    background: var(--muted);
    border-radius: 999px;
    padding: 0.12rem 0.5rem;
  }

  .models-actions {
    display: flex;
    gap: 0.35rem;
    align-items: center;
    flex-shrink: 0;
  }

  :global(.model-search) {
    width: 10rem !important;
    height: 1.8rem !important;
    font-size: 0.78rem !important;
  }

  .model-grid-scroll {
    max-height: 20rem;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.4rem;
  }

  .model-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem;
  }

  .model-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.5rem 0.7rem;
    font-size: 0.8rem;
    color: var(--muted-foreground);
    background: var(--muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .model-card-primary {
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-card-secondary {
    font-size: 0.72rem;
    opacity: 0.78;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-source-badge {
    font-size: 0.68rem;
    line-height: 1;
    padding: 0.18rem 0.4rem;
    border-radius: 999px;
    border: 1px solid currentColor;
    opacity: 0.88;
  }

  .model-card:hover:not(:disabled) {
    border-color: var(--primary);
  }

  .model-card.is-enabled {
    background: var(--primary);
    color: var(--primary-foreground);
    border-color: var(--primary);
  }

  .model-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.manual-model-dialog) {
    max-width: 28rem;
  }

  .manual-model-form {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    padding-top: 0.25rem;
  }

  /* Context Menu */
  .context-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
  }

  .context-menu {
    position: fixed;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.25rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    min-width: 7rem;
    z-index: 101;
  }

  .context-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 0.35rem;
    padding: 0.4rem 0.6rem;
    font-size: 0.82rem;
    cursor: pointer;
    color: var(--foreground);
    transition: background 0.1s;
  }

  .context-item:hover {
    background: var(--muted);
  }

  .context-item.danger {
    color: #dc2626;
  }

  .context-item.danger:hover {
    background: #fef2f2;
  }

  /* Shared */
  .panel-status {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  .alert {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.5rem 0.65rem;
    margin-bottom: 0.7rem;
    font-size: 0.8rem;
  }

  .alert-error {
    border-color: #fecaca;
    background: #fef2f2;
    color: #991b1b;
  }

  .alert-success {
    border-color: #bbf7d0;
    background: #f0fdf4;
    color: #166534;
  }

  .empty-state {
    padding: 3rem 1rem;
    text-align: center;
  }

  .placeholder-panel {
    grid-column: 2 / -1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-state h3 {
    margin: 0;
    font-size: 0.9rem;
  }

  .empty-state p {
    margin: 0.45rem 0 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  /* ── General Settings Panel ── */
  .general-panel {
    grid-column: 2 / -1;
    padding: 1.2rem 1.5rem;
    overflow-y: auto;
    min-height: 0;
  }

  .assistants-panel {
    grid-column: 2 / -1;
    padding: 1.2rem 1.5rem;
    overflow: hidden;
    min-height: 0;
    min-width: 0;
    height: 100%;
    display: flex;
  }

  .combos-panel {
    grid-column: 2 / -1;
    padding: 1.2rem 1.5rem;
    overflow-y: auto;
    min-height: 0;
  }

  .combo-editor {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 1rem;
    margin-bottom: 1rem;
    background: var(--card);
  }

  .combo-field {
    margin-bottom: 0.75rem;
  }

  .combo-label {
    display: block;
    font-size: 0.8rem;
    font-weight: 500;
    margin-bottom: 0.35rem;
    color: var(--foreground);
  }

  /* ---- Combo slots (horizontal card layout) ---- */

  .combo-slots-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: stretch;
  }

  .combo-slot {
    position: relative;
    width: 120px;
    min-height: 64px;
    border-radius: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .combo-slot.filled {
    border: 1px solid var(--border);
    background: var(--card);
    flex-direction: column;
    padding: 0.5rem 0.4rem 0.4rem;
    align-items: stretch;
  }

  .combo-slot.filled.picking {
    border-color: var(--primary);
    box-shadow: 0 0 0 1px var(--primary);
  }

  .combo-slot.dashed {
    border: 2px dashed var(--border);
    background: none;
    cursor: pointer;
    color: var(--muted-foreground);
    font-size: 1.2rem;
  }

  .combo-slot.dashed:hover {
    border-color: var(--primary);
    color: var(--primary);
    background: color-mix(in oklch, var(--primary) 5%, transparent);
  }

  .combo-slot.dashed.picking {
    border-color: var(--primary);
    color: var(--primary);
  }

  .combo-slot-body {
    cursor: pointer;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }

  .combo-slot-body:hover {
    opacity: 0.8;
  }

  .combo-slot-name {
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .combo-slot-provider {
    font-size: 0.68rem;
    color: var(--muted-foreground);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .combo-slot-remove {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--border);
    background: var(--background);
    color: var(--muted-foreground);
    font-size: 0.75rem;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .combo-slot-remove:hover {
    background: var(--destructive);
    color: white;
    border-color: var(--destructive);
  }

  .combo-slot-plus {
    font-weight: 300;
  }

  /* ---- Combo model picker dropdown ---- */

  .combo-picker {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    margin-top: 0.5rem;
    background: var(--card);
    overflow: hidden;
  }

  .combo-picker-title {
    display: block;
    padding: 0.5rem 0.6rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--muted-foreground);
    border-bottom: 1px solid var(--border);
  }

  .combo-picker-list {
    max-height: 200px;
    overflow-y: auto;
    padding: 0.25rem;
  }

  .combo-picker-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.45rem 0.55rem;
    border: none;
    background: none;
    border-radius: 0.3rem;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--foreground);
    text-align: left;
  }

  .combo-picker-item:hover {
    background: var(--muted);
  }

  .combo-picker-model-name {
    flex: 1;
  }

  .combo-picker-model-provider {
    color: var(--muted-foreground);
    font-size: 0.72rem;
  }

  .combo-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 0.75rem;
  }

  .combo-empty-state {
    text-align: center;
    color: var(--muted-foreground);
    padding: 2rem;
    font-size: 0.85rem;
  }

  .combo-list-settings {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .combo-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--card);
  }

  .combo-row-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .combo-row-name {
    font-weight: 500;
    font-size: 0.85rem;
    color: var(--foreground);
  }

  .combo-row-count {
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  .combo-row-actions {
    display: flex;
    gap: 0.35rem;
  }

  .general-switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  /* ── Color Swatches ── */
  .color-swatches {
    display: flex;
    gap: 0.6rem;
    margin-top: 0.5rem;
    flex-wrap: wrap;
  }

  .color-swatch {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    transition: border-color 0.15s, transform 0.15s;
  }

  .color-swatch:hover {
    transform: scale(1.12);
  }

  .color-swatch.is-selected {
    border-color: var(--foreground);
    box-shadow: 0 0 0 2px var(--background), 0 0 0 4px var(--foreground);
  }

  /* ── Zoom ── */
  .zoom-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
  }

  .zoom-value {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--muted-foreground);
  }

  :global(.zoom-slider) {
    margin-top: 0.25rem;
  }

  /* ── Data Settings ── */
  .section-subtitle {
    display: block;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--foreground);
    margin-bottom: 0.7rem;
  }

  .data-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    padding: 0.45rem 0;
  }

  .data-row-label {
    font-size: 0.82rem;
    color: var(--foreground);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .data-row-content {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .data-path {
    font-size: 0.75rem;
    color: var(--muted-foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 22rem;
  }

  .data-value {
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  .form-input-sm {
    width: 8rem;
    padding: 0.3rem 0.5rem;
    font-size: 0.8rem;
  }

  :global(.input-sm) {
    width: 5rem !important;
    height: 1.8rem !important;
    font-size: 0.8rem !important;
  }

  /* ── About ── */
  .about-hero {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 1.5rem 0 1rem;
    gap: 1rem;
  }

  .about-title-block {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .about-app-name {
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--foreground);
    letter-spacing: -0.01em;
  }

  .about-tagline {
    font-size: 0.88rem;
    color: var(--muted-foreground);
  }

  .about-version {
    font-size: 0.78rem;
    color: var(--muted-foreground);
    opacity: 0.7;
  }

  .github-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.38rem 0.85rem;
    border-radius: 0.4rem;
    border: 1px solid var(--border);
    background: var(--card);
    color: var(--foreground);
    font-size: 0.82rem;
    font-weight: 500;
    text-decoration: none;
    transition: background 0.15s, border-color 0.15s;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .about-update-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .update-status {
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  .update-status.is-error {
    color: var(--destructive);
  }

  .github-badge:hover {
    background: var(--accent);
    border-color: var(--primary);
  }

  .about-update-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .update-status {
    font-size: 0.82rem;
    color: var(--muted-foreground);
  }

  /* Responsive */
  @media (max-width: 1160px) {
    .settings-root {
      grid-template-columns: 11.5rem 12rem minmax(0, 1fr);
    }

    .model-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 980px) {
    .settings-root {
      grid-template-columns: 1fr;
    }

    .settings-nav,
    .provider-list-panel {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }

    .settings-nav {
      max-height: 15rem;
    }

    .provider-list-panel {
      max-height: 20rem;
    }

    .key-input-row {
      grid-template-columns: 1fr;
    }

    .model-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
