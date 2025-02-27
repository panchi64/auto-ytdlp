# Auto-YTDLP Implementation Checklist

Based on your feedback, here's a checklist of improvements to implement:

## User Experience Enhancements

- [ ] **Interface Improvements**

  - [x] Improve the keyboard shortcuts area in the current TUI
  - [ ] Show available space in the drive that the downloads will be placed in within the bottom-right of the screen
  - [x] Add color-coding for log messages (errors in red, warnings in yellow, etc.)
  - [ ] Automatically clear logs when downloads complete or are stopped
  - [ ] Add visual indicators for paused/active/failed downloads on the title bar

- [ ] **Configuration Options**

  - [ ] Create a settings menu accessible via keybind
  - [ ] Add format presets (audio-only, specific resolutions) to settings menu
  - [ ] Implement persistent user preferences across sessions

- [ ] **Notifications**
  - [ ] Implement fallback notification methods when desktop notifications fail

## Performance Optimizations

- [ ] **Resource Management**

  - [ ] Add disk space checking before downloads start (display on bottom left of TUI)
  - [ ] Implement bandwidth throttling options in settings menu
  - [ ] Add configurable automatic retry logic for network failures
  - [ ] Clean up completed tasks from memory

- [ ] **Concurrency Improvements**

  - [ ] Add concurrency adjustment to settings menu
  - [ ] Create worker pool only when downloads start (not at script startup)

- [ ] **Process Handling**
  - [ ] Add timeout handling for stalled downloads
  - [ ] Implement more robust process termination in force-quit mode
  - [ ] Improve signal handling for cleaner shutdown

## Code Structure and Maintainability

- [x] **Architecture Refinements**

  - [x] Create a dedicated download manager module
  - [x] Refactor state management to reduce mutex complexity

- [ ] **Error Handling**

  - [ ] Replace unwrap() and expect() calls with proper error handling
  - [ ] Implement consistent error types throughout the application
  - [ ] Add better error recovery mechanisms

- [ ] **Documentation and Testing**

  - [ ] Add comprehensive documentation for all public functions
  - [ ] Expand test coverage, especially for edge cases
  - [ ] Create integration tests for common user workflows

- [ ] **Code Quality**

  - [ ] Break down larger functions into smaller, more testable units
  - [ ] Reduce code duplication, especially in the download workflow

- [ ] **Dependencies**
  - [ ] Check for dependencies in a more user-friendly way
