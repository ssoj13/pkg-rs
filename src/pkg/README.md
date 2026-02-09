# pkg (CLI)

## What It Is
Command-line interface for pkg-rs. Provides environment setup, package inspection, repository scanning, build execution, and pip import.

## How It Works
- Delegates core logic to `pkg_lib`.
- Loads package definitions (`package.py`), resolves dependencies, and emits shell/env output.
- `pkg build` runs the build pipeline in the current package directory.
- `pkg pip` installs a Python package into a repository and generates a `package.py` wrapper.

## Команды: passthrough и ограничения

**Полный синтаксис (как rez), без добавления флагов по одному:**

| Команда | Как устроено | Ограничения / что можно так же |
|--------|----------------|--------------------------------|
| **pip** | После имени пакета все аргументы передаются в `pip install` (trailing_var_arg). Флаги: -i, -r, -p, --python-version, -e; ещё аргументы после `--`. | — Сделано. |
| **build** | Аргументы для build-системы: `--build-args "…"` или всё после `--` (extra_args). | Уже есть; при желании можно добавить trailing (всё после `build` до следующего известного флага) как у pip, но rez-build тоже использует `--` для build-args. |
| **env** | Команда для запуска — всё после `--`. Пример: `pkg env maya -- maya.exe`. | — |
| **search** | Список/поиск пакетов: `pkg search`, `pkg search -L`, паттерны, `-t` tag, `--json`. | — |
| **view** | Детали пакета: `pkg view maya`, `pkg view maya --json`. | — |
| **depends** | Граф зависимостей: `pkg depends maya` (-f dot/mermaid, -R, -d). | — |
| **bind** | Clap BindArgs: -l, -s, --quickstart, -r, -i, &lt;name&gt;; `pkg bind --help`. | — |
| **test** | Аргументы теста — после `--` (extra_args). | — Норма. |
| **bind** | Полный разбор через clap (BindArgs): -l/--list, -s/--search, --quickstart, -r, --no-deps, -i, пакет. | Сделано: `pkg bind --help` выводит справку. |
| **release** | extra_args после `--` передаются в build. | Как build. |

**Идея:** там, где команда по сути оборачивает внешнюю утилиту (pip, cmake, make), лучше один раз сделать «всё после X пробрасывается» (trailing или после `--`), чем плодить свои флаги под каждый вариант.

## Current Status
- Build command supports Rez-style build variables, variants, hashed variants, `pre_build_commands`, and modular build systems (`custom`, `make`, `cmake`, `cargo`, `python`).
- Pip command matches rez-pip install layout (dist-info parsing, entry-point wrappers, hashed variants) and bundles dependencies when multiple dist-info are present.
- Repo configuration follows Rez config (`rezconfig.py`, `REZ_CONFIG_FILE`, `~/.rezconfig`) for scan paths and install targets.
- Some Rez-only flows (rezified pip/python context, release hooks, suite context) are not implemented.
