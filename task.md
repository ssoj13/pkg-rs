# pkg-rs GUI Implementation Plan

## Overview
GUI for pkg-rs via `pkg -g` / `pkg --gui` flag.

## Directory Structure

```
~/.pkg-rs/
  packages/              # user packages (auto-added to locations)
    .toolsets/           # user toolsets
      my-env.toml
      vfx-2024.toml
  config.toml            # GUI settings

repo/
  .toolsets/             # project toolsets
    studio.toml
```

## Dependencies (Cargo.toml)

```toml
eframe = { version = "0.33", features = ["persistence"] }
egui-snarl = { version = "0.9", features = ["serde"] }
egui_extras = "0.33"
egui_ltreeview = "0.6"
```

## File Structure

```
src/
  gui/
    mod.rs              # PkgApp, eframe::App impl
    state.rs            # AppState, Selection, UIMode
    package_list.rs     # left panel (packages/toolsets list)
    tree_editor.rs      # TreeView: Package -> Envs -> Evars (+/- buttons)
    node_graph.rs       # egui-snarl dependency graph
    toolset_editor.rs   # create/edit toolsets
    actions.rs          # Launch, Solve, Export
```

## Core Changes

### storage.rs
- Add `~/.pkg-rs/packages` to default_locations()

### toolset.rs
- Add `save_toolset(path, name, def)` for writing TOML

### env.rs
- Add export methods:
  - `to_cmd()` -> `SET VAR=value`
  - `to_ps1()` -> `$env:VAR = "value"`
  - `to_sh()` -> `export VAR="value"`
  - `to_py()` -> `os.environ['VAR'] = 'value'`

### package.rs (optional)
- `Package.launch(app_name, resolve=true, storage)` with auto-resolve

## UI Layout

```
+------------------------------------------------------------------+
|  Mode: [Packages v]  [Toolsets]     [Settings]                   |
+----------------+--------------------------------------------------+
|  maya-2026     |  [Tree] [Graph]                                  |
|  houdini-20    |  ------------------------------------------------|
|  redshift-3    |  v maya-2026.1.0                                 |
|  vfx-2024      |    +- envs                                       |
|                |    |   v default                                 |
|  [+] New       |    |      +- PATH: /opt/maya/bin        [-]     |
|                |    |      +- [+]                                 |
|                |    +- apps                                       |
|                |    |   v maya                     [> Launch]    |
|                |    |      +- path: /opt/.../maya                 |
|                |    +- reqs: [redshift@>=3.5]      [+]           |
|                |    +- tags: [dcc, autodesk]       [+]           |
|                |  ------------------------------------------------|
|                |  [Solve]  [Export v]                             |
+----------------+--------------------------------------------------+
```

## Node Graph View
- Depth slider: `[0 ----*---- max]` (max computed per selection)
- Collapse/expand via double-click
- Node colors by tags:
  - `toolset` -> blue
  - `dcc` -> green
  - `render` -> orange

## Actions

| Action   | Description                                    |
|----------|------------------------------------------------|
| Solve    | `pkg.solve(storage)` -> show resolved deps     |
| Launch   | `app.launch(env)` with auto-resolve if needed  |
| Export   | Save env as .cmd / .ps1 / .sh / .py            |

## Implementation Order

1. [x] Add export methods to Env (to_cmd, to_ps1, to_sh, to_py)
2. [x] Add ~/.pkg-rs/packages to Storage with -u/--user flag
3. [x] Add save_toolset() to toolset.rs
4. [x] Create gui/ module structure
5. [x] Implement PkgApp with basic eframe setup
6. [x] Package list panel (left)
7. [x] Tree editor (right, default view)
8. [x] Node graph view with depth slider (basic text version)
9. [x] Actions: Solve, Launch, Export (UI ready)
10. [x] Add -g/--gui flag to CLI

## Usage

```bash
# Build
cargo build

# Run GUI
pkg -g
pkg --gui

# With user packages
pkg -ug
```

## Completed Improvements

- [x] Full egui-snarl node graph with visual nodes (depth slider, color by tags)
- [x] Toolset editor with create/edit/delete via modal dialog
- [x] Save/Load dialogs for Export (rfd crate)
- [x] Solve with 3-column result display (packages, apps, merged env)
- [x] Launch with resolved environment from dependencies

## TODO (future improvements)

- [ ] Drag & drop reordering in toolset editor
- [ ] Launch confirmation dialog
- [ ] Process management (running apps list)
