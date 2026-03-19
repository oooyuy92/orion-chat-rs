# Shadcn Theme Presets Design

Replace the current "theme color" feature with full `shadcn/ui` theme presets from the official Themes page.

## Problem

The current display setting only updates `--primary` and `--primary-foreground` in `ProviderSettings.svelte`. The rest of the semantic tokens in `src/app.css` stay neutral white/gray, so the app still looks washed out even after picking a color.

## Goal

Provide eight switchable light theme presets that apply globally across the app:

- `default`
- `blue`
- `green`
- `orange`
- `red`
- `rose`
- `violet`
- `yellow`

The selected theme must persist in `settings.json` and be applied on app startup before the user opens Settings again.

## Solution

Use a single global theme key stored as `theme` in `settings.json`, then map that key to a complete CSS token preset through `:root[data-theme="<name>"]` selectors in `src/app.css`.

## Architecture

### Theme Source of Truth

Create a focused frontend theme module that owns:

- the official theme keys
- theme metadata for the settings UI
- normalization and fallback logic
- DOM application via `document.documentElement.dataset.theme`

This replaces the current `colorIndex`-based swatch logic.

### CSS Token Application

Keep the existing semantic CSS variable names:

- `--background`
- `--foreground`
- `--card`
- `--popover`
- `--primary`
- `--secondary`
- `--muted`
- `--accent`
- `--border`
- `--input`
- `--ring`
- `--sidebar`
- `--surface`

`src/app.css` becomes the global theme host:

- `:root` defines the default preset
- `:root[data-theme="blue"]` through `:root[data-theme="yellow"]` override the full semantic token set

This ensures buttons, surfaces, sidebars, dialogs, cards, and hover states all shift together.

### Startup Initialization

`src/routes/+layout.svelte` loads the saved `theme` from `settings.json` and applies it on mount. When no saved value exists or the value is invalid, it falls back to `default`.

### Settings UI

`ProviderSettings.svelte` keeps the theme picker in Display Settings, but the picker changes from custom color dots to official theme presets:

- one option per official theme
- immediate global apply on click
- persisted through the existing settings autosave flow

The UI should preview the theme as a small surface card, not a single primary-color dot, so users can see the difference in background, border, and accent behavior.

## Data Model

Persist one field in `settings.json`:

```json
{
  "theme": "blue"
}
```

Remove dependence on:

- `colorIndex`

Old `colorIndex` values can simply be ignored. No migration is needed because the new default is deterministic.

## Testing

Add frontend tests that verify:

- only the eight official theme keys are exported
- invalid theme keys normalize to `default`
- applying a theme writes the root `data-theme` attribute
- `src/app.css` defines selectors for all eight presets

## Files to Modify

| File | Change |
|------|--------|
| `src/app.css` | Define default preset and seven `data-theme` overrides |
| `src/routes/+layout.svelte` | Load and apply saved theme on startup |
| `src/lib/components/settings/ProviderSettings.svelte` | Replace `colorIndex` swatches with official theme preset picker |
| `src/lib/utils/theme.js` | Add theme keys, metadata, normalization, and DOM apply helper |
| `src/lib/utils/theme.test.js` | Add unit tests for theme helpers |
| `src/lib/themeCssContract.test.js` | Add CSS contract test for preset selectors |
