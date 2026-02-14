# ui/ Module

Terminal User Interface using ratatui and crossterm.

## Structure

### tui/
The main TUI implementation, split into submodules:

- **mod.rs**: `run_tui()` main event loop, `UiContext` struct for UI-only state (not in AppState)
- **render.rs**: All rendering functions - `ui()` coordinates panels, helpers for progress bars, help overlay, toast
- **input.rs**: Keyboard event handlers split by mode (normal, edit, help overlay)

### settings_menu.rs
F2 overlay for configuring download options. Uses `SettingsMenu` struct with:
- `selected_option`: Current highlighted setting
- `edit_mode`: Whether currently editing a value
- Option cycling with Up/Down arrows when editing

## Key Patterns

### UiSnapshot
The TUI captures all state once per frame via `state.get_ui_snapshot()`. This single lock acquisition avoids multiple mutex locks during rendering.

### Input Handling Flow
```
Key Event → settings_menu.handle_input() → help_overlay → edit_mode → normal_mode
```
Each handler returns whether it consumed the event.

### InputResult Enum
- `Continue`: Event handled, continue loop
- `Break`: Exit TUI loop
- `Unhandled`: Pass to next handler (e.g., F2 for settings)

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| S | Start/Stop downloads |
| P | Pause/Resume |
| Q | Graceful quit |
| Shift+Q | Force quit (2-press confirm) |
| E | Queue edit mode |
| A | Add clipboard URLs |
| R | Reload links.txt |
| F | Load from file |
| F1 | Help overlay |
| F2 | Settings menu |
