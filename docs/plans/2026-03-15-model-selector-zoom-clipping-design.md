# Model Selector Zoom Clipping Design

**Problem**

When the page is zoomed and the chat composer sits near the bottom of the viewport, the model selector dropdown can render downward and become partially hidden. The menu content already renders through a portal, so the issue is floating placement under constrained viewport height rather than ancestor clipping.

**Options**

1. Force the model selector menu to open upward from the composer trigger.
2. Keep default placement and rely on collision detection to flip when needed.
3. Replace the dropdown with a larger popover or sheet for constrained layouts.

**Recommendation**

Use option 1. This trigger always lives in the bottom composer row, so opening upward is the most stable behavior across zoom levels and small viewports. Add a small collision padding so the menu keeps distance from viewport edges.

**Scope**

- Update `src/lib/components/chat/ModelSelector.svelte` to pass explicit placement props to `DropdownMenuContent`.
- Add a source-level contract test proving the placement props are present.
- Verify with the targeted test and the existing project check command.
