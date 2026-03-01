# Orion Chat Shadcn Chatbot Style Design

## Context
- Existing app is SvelteKit + Tailwind v4 with a sidebar + chat layout.
- Existing behavior for conversations, model loading, and streaming events is stable and should be retained.
- Target style and interaction reference is https://www.shadcn.io/ai/chatbot.

## Confirmed Constraints
- Keep the existing left sidebar and settings entry.
- Default to light theme.
- No dark-mode adaptation in this scope.
- Use Svelte implementation (no React migration).

## Goals
- Align visual language with shadcn chatbot: spacing, typography rhythm, borders, muted surfaces, and button hierarchy.
- Align interaction structure with shadcn chatbot: suggestion pills, composer tool row, and cleaner message stack behavior.
- Preserve current backend API usage and streaming state transitions.

## Architecture
- Keep existing high-level route (`src/routes/+page.svelte`) but reorganize rendered structure into:
  - Sidebar region (retained feature set, refreshed visual style).
  - Chat top bar region.
  - Conversation viewport region.
  - Prompt composer region with suggestion pills and controls.
- Rework existing components instead of introducing framework-specific external UI dependencies.

## Component Design
### `+page.svelte`
- Keep state model and event handling for streaming.
- Move model selector from classic `<select>` placement to shadcn-style control surfaces.
- Define local suggestion prompts and pass them to composer.

### `MessageList.svelte`
- Build a scrollable conversation viewport with centered content width and improved empty state.
- Keep autoscroll behavior while preserving manual read ergonomics.

### `MessageBubble.svelte`
- User messages: right aligned, compact muted bubble.
- Assistant messages: left aligned with typography-forward content styling.
- Reasoning block as collapsible muted panel with clearer hierarchy.
- Keep token display as subdued metadata.

### `InputArea.svelte`
- Rebuild as prompt composer:
  - Suggestion chips above input area.
  - Rounded bordered composer container.
  - Multi-line textarea with enter-to-send and shift+enter newline.
  - Tool row with icon-like controls and model selector button.
  - Compact primary submit button.

### `ConversationList.svelte`
- Refresh list visuals to light shadcn-style muted panels and hover states.
- Keep behavior for create/select/delete conversations.

### `ModelSelector.svelte`
- Convert to subtle button-like select styling consistent with shadcn control pattern.

### `ProviderSettings.svelte`
- Keep behavior and structure, align to updated global tokens so visual style remains coherent.

## Visual Tokens
- Replace current accent-heavy variables with neutral light tokens:
  - background/foreground
  - card/muted/muted-foreground
  - border/input
  - primary/primary-foreground
  - radius
- Apply in `src/app.css` and rely on utility classes + minimal component-scoped styles.

## Behavior and Data Flow
- Preserve all existing API entry points and event handlers.
- Preserve optimistic user message insertion and assistant placeholder streaming updates.
- Preserve reasoning aggregation and token usage display.
- For incomplete context (no active conversation or no models), show clean inline guidance.

## Error Handling
- Keep existing console logging and state guards.
- Surface user-facing guidance using muted inline text rather than high-contrast warning styles.

## Testing and Verification
- Run static/type checks with `pnpm check`.
- Manual verification checklist:
  - conversation create/select/delete works
  - send message and streaming updates render correctly
  - reasoning toggle works
  - suggestion chips insert/send content as expected
  - model selector remains functional
  - layout works in desktop and narrow viewport

## Out of Scope
- Dark theme adaptation.
- Backend protocol or API changes.
- Feature additions like citations, attachments, or branching navigation.
