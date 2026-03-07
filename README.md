# Orion Chat

Orion Chat is a desktop AI chat client built with **Tauri v2**, **SvelteKit**, and **Rust**. It provides a local-first multi-provider chat workspace with conversation management, model switching, message versioning, and desktop-friendly settings/data tools.

## Highlights

- Multi-provider support: OpenAI-compatible APIs, Anthropic, Gemini, and Ollama
- Local desktop app built with Tauri v2 and a Rust backend
- Conversation history stored locally with SQLite + full-text search
- Streaming responses with message regeneration and version switching
- Assistant presets and per-model parameter configuration
- Conversation auto-rename and auto-compress workflows
- Markdown export / JSON export, local backup, cache cleanup, and app data reset
- Responsive Svelte 5 UI with shadcn-svelte style components

## Tech Stack

- Frontend: SvelteKit, Svelte 5, Tailwind CSS 4
- Desktop shell: Tauri v2
- Backend: Rust, Tokio, Reqwest, Rusqlite
- Storage: SQLite + FTS5

## Project Structure

- `src/` — SvelteKit frontend, UI components, stores, and routes
- `src-tauri/src/commands/` — Tauri command handlers for chat, providers, settings, export, and search
- `src-tauri/src/db/` — SQLite schema and persistence layer
- `src-tauri/src/providers/` — provider integrations
- `docs/plans/` — design notes and implementation plans

## Features

### Chat workflow

- Start and manage multiple conversations
- Stream assistant replies in real time
- Regenerate responses and switch between message versions
- Soft-delete and restore messages
- Search messages with local full-text indexing

### Provider management

- Add, edit, enable, disable, and delete providers
- Fetch provider model lists and control model visibility
- Configure model parameters for different providers

### Desktop settings

- Theme color and zoom controls
- Auto-launch and proxy mode settings
- Backup location selection and local database backup
- Open app data / log directories from the UI
- Clear cache and reset local app data

## Development

### Requirements

- Node.js
- `pnpm`
- Rust toolchain
- Tauri prerequisites for your OS

### Run locally

```bash
pnpm install
pnpm tauri dev
```

### Type check frontend

```bash
pnpm check
```

## Status

This repository is being actively developed.
