# Crock ⏱️

Crock is a modern, terminal-based Pomodoro and Task Management tool built with Rust and [Ratatui]. It helps you stay focused with a clean, high-visibility clock and an integrated task queue.

## Features

- **Big Text Timer**: High-visibility countdown powered by `tui-big-text`.
- **Task Management**: Create, edit, and reorder your task queue.
- **Dynamic Status**: Visual indicators for running, paused, and upcoming tasks.
- **Flexible Time Parsing**: Input time in natural formats like `25m`, `1h 30m`, or `45s`.
- **System Notifications**: Get alerted when a task timer finishes.
- **Modern UI**: Clean layout with rounded borders and intuitive color coding.

## Installation

```bash
# Clone the repository
git clone https://github.com/your-username/crock.git
cd crock

# Build and run
cargo run --release
```

## Keybindings

Crock uses context-aware keybindings to keep the interface clean.

### 🕒 Clock View (Main)
- `p`: Pause / Resume the timer
- `r`: Start / Restart the current task
- `t`: Stop the current task
- `e`: Switch to **Task List** management
- `?`: Toggle **Help** pane
- `q`: Quit application

### 📝 Task List
- `a`: Add a new task (opens dialog)
- `r`: Edit the focused task
- `d`: Delete the focused task
- `j` / `k`: Move focus down / up
- `Enter`: Set the focused task as the **Current Task**
- `Esc` / `q`: Return to **Clock View**

### ⌨️ Task Input Dialog
- `Tab`: Switch between **Description** and **Duration** fields
- `Enter`: Confirm and save task
- `Esc`: Cancel and return to list

## Interface

The UI is divided into several logical sections:
1. **Status Bar**: Shows the app name and current state (RUNNING/PAUSED).
2. **Main Timer**: Displays the current task name and a large countdown.
3. **Progress Bar**: Visual representation of the remaining time.
4. **Next Up**: A preview of the next task in your queue.
5. **Footer**: Quick-reference keybindings for the current context.

## License

Copyright (c) Charon Ryui <Charon_Ryui@outlook.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[Ratatui]: https://ratatui.rs
[LICENSE]: ./LICENSE