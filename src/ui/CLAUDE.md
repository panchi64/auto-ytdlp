# ui/ Module

Terminal User Interface using ratatui and crossterm.

## Structure

### tui/
The main TUI implementation:

- **mod.rs**: `flip_index()`, `url_key()`, `FLASH_DURATION` and the module wiring
- **context.rs**: `UiContext`, the UI-only state that is not in AppState, plus the flash tracking and scroll helpers
- **run.rs**: `run_tui()` main event loop
- **render/**: `mod.rs` lays out the frame; `panels/{pending,active,logs_footer}.rs` draw each panel; `overlays.rs` has the help overlay and toast; `style.rs` holds shared colour and formatting helpers
- **input/**: `normal.rs`, `edit.rs`, `filter.rs` and `mouse.rs` handle one mode each; `actions/downloads.rs` and `actions/links.rs` hold what the keys do. The root keeps `DownloadState`, `ForceQuitState`, `InputResult` and `NormalModeContext`.

### settings_menu/
F2 overlay for configuring download options.

**`table.rs` is the single place to add a setting.** The `SETTINGS` descriptor array
carries each item's label, description, kind (`Bool`/`Enum`/`Text`/`Action`) and its
audio-only override. It replaced five separate `match` blocks over the menu index, so
adding a setting used to mean editing five functions. Only the enum read/write arms in
`input.rs` and `apply.rs` still need per-setting code, because each maps a distinct type.

The rest: `mod.rs` (the `SettingsMenu` struct, `persist`), `input.rs` (navigation),
`edit.rs` (value cycling), `apply.rs` (writing the setting back), `display.rs` (list item
formatting), `render/` (the list and its popups).

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

### Pending List Display Order
The queue stays FIFO - workers pop the oldest URL with `pop_queue()` - but the
pending panel draws it **newest first**, so links added while the user is watching
appear at the top. `flip_index()` in `tui/mod.rs` converts between the two, and is
its own inverse. Anything in `UiContext` that names a row (`queue_selected_index`,
`queue_scroll`) is in display order; anything passed to `AppState` is in queue order.

### Pending List Scrolling
`UiContext::queue_scroll` is the first visible display row. Each frame
`render_pending_queue()` records what it drew - `queue_view_height`, `queue_drawn_len`,
and `queue_area` - and the input handlers read those back. This is deliberate: edit
mode maps rows with `queue_drawn_len`, not the live queue length, so a link appended
between the frame and the keypress cannot make `D` delete a row the user never saw.

Scroll with ↑↓, PgUp/PgDn, Home/End, or the mouse wheel. `handle_mouse_input()`
hit-tests against `queue_area`, and in edit mode the wheel moves the *selection*
rather than the viewport so the highlighted row can never scroll out of sight.

### New Link Flash
`UiContext::track_new_links()` diffs the queue snapshot against the previous frame
and timestamps anything new, which `render_pending_queue()` renders as a fading
background for `FLASH_DURATION`. URLs are keyed by hash, not cloned, since this runs
on every frame. The first frame only records a baseline so links restored from
`links.txt` at startup do not flash.

Because new links enter at the top of the panel, they push everything below them
down. `track_new_links()` compensates by shifting `queue_scroll` (and the edit-mode
selection) so a user who has scrolled away stays on the links they were reading; at
the top the view stays put so new links remain visible.

The fade needs truecolor, so `flash_style()` falls back to basic ANSI when the ASCII
indicator setting is on.

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
| ↑ ↓ / wheel | Scroll pending list |
| PgUp/PgDn/Home/End | Scroll pending list further |
| F1 | Help overlay |
| F2 | Settings menu |
