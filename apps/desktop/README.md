# desktop

A Tauri shell for open-slide. On first launch it asks where your slide
project should live, initializes it via `npx @open-slide/cli init` if the
folder is empty, runs the project's dev server itself, and shows it in the
window. The toolbar detects coding agents installed on the machine and
launches one in the project folder.

## Requirements

- The [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your
  platform (Rust toolchain, plus webkit2gtk/gtk on Linux).
- Node.js on the machine that runs the app (the app shells out to
  `node`/`npx` to init projects and run the dev server).

## Develop

```bash
pnpm core build      # once, so the workspace demo can run
pnpm dev:desktop     # from the repo root
```

Debug builds default the project folder to `apps/demo` so the app dogfoods
the workspace demo; release builds ask on first launch.

## Build

```bash
pnpm build:desktop
```

## How it works

- **Project folder** — chosen via a native folder picker on first run and
  persisted in the app config dir; override with `OPEN_SLIDE_PROJECT_DIR`.
  Change it any time with the folder button in the toolbar.
- **Init** — if the chosen folder has no `package.json`, the app runs
  `npx -y @open-slide/cli@latest init . --use-npm` in it, streaming output
  into the UI. Nothing is bundled; the live CLI does the scaffolding.
- **Dev server** — the app spawns `node node_modules/@open-slide/core/bin.js
  dev --port <free port>` for the project (running `npm install` first if
  needed), embeds it, and kills it on exit.
- **Agents** — the Rust side scans `PATH` plus well-known install dirs
  (`~/.local/bin`, `~/.claude/local`, `/opt/homebrew/bin`, …) for Claude
  Code, Codex CLI, Gemini CLI, Copilot CLI, Cursor Agent, opencode, and
  Aider. **Launch here** opens the system terminal in the project folder
  running the selected agent.
