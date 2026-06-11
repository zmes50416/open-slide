# desktop

A Tauri shell around the open-slide demo, with a toolbar that detects coding
agents installed on the machine and launches one in the slide project folder.

## Requirements

- The [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your
  platform (Rust toolchain, plus webkit2gtk/gtk on Linux).

## Develop

```bash
pnpm dev:desktop   # from the repo root
```

This starts the demo dev server (port 5173) and the toolbar frontend
(port 1420), then opens the Tauri window embedding both.

## Build

```bash
pnpm build:desktop
```

Builds the toolbar frontend, builds the demo as a static site under
`dist/slides/` (base `/slides/`), and bundles everything into a native app.

## Agent integration

The Rust backend scans `PATH` plus well-known install directories
(`~/.local/bin`, `~/.claude/local`, `/opt/homebrew/bin`, …) for known agent
CLIs: Claude Code, Codex CLI, Gemini CLI, Copilot CLI, Cursor Agent, opencode,
and Aider. The **Launch here** button opens the system terminal in the slide
project folder running the selected agent.

The project folder defaults to `apps/demo` in dev builds and the current
working directory in release builds; override it with the
`OPEN_SLIDE_PROJECT_DIR` environment variable.
