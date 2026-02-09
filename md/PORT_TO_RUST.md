# Портирование на Rust — полный обзор

Дата: 2026-02-08

Документ перечисляет **все** места, где pkg-rs вызывает Python или зависит от кода Rez в `python/rez`, и даёт план портирования на Rust с приоритетами.

---

## 1. Карта границ Python

| Компонент | Файл | Что делает | Зависимость от Rez |
|-----------|------|------------|--------------------|
| **Config** | `config.rs` | Загрузка конфига: выполнение rezconfig.py (и override файлов), извлечение переменных в JSON (ModifyList, DelayLoad, _obj_dict, json.dumps). Применение REZ_* и REZ_*_JSON. | Выполняет **любой** rezconfig.py (Python); нужны классы ModifyList, DelayLoad и хелпер `_extract` для конвертации модуля в dict→JSON. |
| **Solver (Rez backend)** | `solver/mod.rs` | При `resolver_backend=rez`: rez.config.Config(filepaths), _replace_config, ResolvedContext(requirements), чтение resolved_packages. | **rez.config**, **rez.resolved_context** (полный Rez solver + репозитории в Python). |
| **Loader** | `loader.rs` | Выполнение package.py: globals (pkg, Package, Env, …), py.run(code), get_package(), извлечение pre_commands и т.д. | Нужен **интерпретатор Python** для выполнения произвольного package.py. |
| **Rex (env)** | `rex.rs` | Выполнение pre_commands/commands/post_commands: bootstrap _RexEnv, py.run(source), сбор _evars, применение к Env. | Только **стандартная библиотека** (os, sys); логика rex — наш код в строке. |
| **Bind (Python strategy)** | `bind_module.rs` | Для модулей python, rez, setuptools, pip: rez.package_bind.bind_package(name, path=…). | **rez.package_bind** (пишет package.py в path, может вызывать системный python/pip). |
| **Build: parse_build_args** | `build.rs` | Выполнение parse_build_args.py из пакета: argparse, parser.add_argument из кода, parse_known_args, vars(), shlex.quote → env REZ_BUILD_*. | **Любой** parse_build_args.py пакета (Python); stdlib argparse, shlex. |
| **Build: pre_build_commands** | `build.rs` | Rex-подобное выполнение pre_build_commands: bootstrap _BuildEnv, this/build/install, py.run(source), вызов pre_build_commands(), чтение _evars. | Только **stdlib**; семантика как в rex.rs. |
| **Pip: requirements** | `pip.rs` | Парсинг Requires-Dist: packaging.requirements.Requirement, packaging.markers.Marker.evaluate → (name, specifier, marker). | Внешняя библиотека **packaging** (PEP 508); есть fallback без Python. |
| **Storage/Repo** | `storage.rs`, `repo/*.rs` | Инициализация Python (для Loader при скане). | Только **Python::initialize**; логика скана — Rust. |
| **Package (help)** | `package.rs` | Optional: inspect.getsource(help_obj) для секции help. | stdlib **inspect**. |
| **CLI: pkg py** | `pkg/python.rs` | REPL или запуск скрипта: sys.path, pkg module, run(code). | Нужен **интерпретатор** для интерактива и скриптов. |

---

## 2. Зависимости от дерева python/rez

Что реально импортируется из `python/rez`:

| Импорт | Где используется | Можно заменить на Rust? |
|--------|------------------|--------------------------|
| **rez.config** | Solver (Rez backend): Config, _replace_config | Да: конфиг уже грузим мы в config.rs; Rez backend можно перевести на наш Config или портировать solver. |
| **rez.resolved_context** | Solver (Rez backend): ResolvedContext(reqs) | Да: порт алгоритма из rez/solver.py + rez/resolver.py (см. SOLVER.md) или использование rez-next-solver через адаптер. |
| **rez.package_bind** | Bind (Python strategy): bind_package, _print_package_list | Частично: для python/rez/setuptools/pip можно портировать логику bind (поиск exe, версия, запись package.py) в Rust; сложность — средняя. |
| **rez.config (схема)** | Не вызывается напрямую; Rez использует для валидации | Опционально: валидация по схеме в Rust по образцу rez/config.py. |

Остальное (loader, rex, build pre_build, config load) либо выполняет **произвольный пользовательский код** (package.py, rezconfig.py, parse_build_args.py), либо наш собственный мини-код (rex bootstrap, config bootstrap). Портирование «всего» значит:

- **Конфиг** — перестать выполнять rezconfig.py в Python; парсить и мержить в Rust.
- **Solver Rez** — порт алгоритма Rez в Rust (или один интерфейс + rez-next).
- **Loader** — либо оставить выполнение package.py (тогда Python обязателен), либо ввести опциональный парсер package.py (AST/статический) в Rust для «только метаданные без кода».
- **Rex** — уже наш код; можно оставить py.run для совместимости или портировать интерпретатор команд в Rust (подмножество Python).
- **Bind Python** — портировать по одному модулю (python, rez, setuptools, pip) в Rust.
- **Build parse_build_args** — либо минимальный парсер аргументов в Rust по конвенции Rez, либо оставить Python для произвольных скриптов.
- **Pip requirements** — парсер PEP 508 в Rust (есть крейты, например `pep508_rs` или аналог); fallback уже есть.

---

## 3. Приоритеты портирования

### Высокий приоритет (убираем зависимость от Rez для типичного сценария)

1. **Config загрузка**  
   - **Сейчас:** выполнение rezconfig.py в Python, извлечение в JSON через _extract.  
   - **Порт:** парсинг rezconfig.py как конфига (ограниченное подмножество: присваивания, списки, словари, ModifyList/DelayLoad как специальные типы). Либо формат «только YAML/JSON» для pkg-rs с опцией «если есть rezconfig.py — вызвать Python» для обратной совместимости.  
   - **Результат:** один бинарник без Rez для конфига (пути, resolver_backend, plugins.*).

2. **Rez solver в Rust**  
   - **Сейчас:** при backend=rez вызывается Python ResolvedContext.  
   - **Порт:** перенести алгоритм из `python/rez/solver.py` и `resolver.py` в Rust (один бэкенд, реализующий трейт `Resolver`), либо адаптер к rez-next-solver.  
   - **Результат:** переключение resolver_backend=rez без Python.

### Средний приоритет

3. **Bind (Python-модули)**  
   - Порт `bind_package` для python, rez, setuptools, pip: определение пути к exe, версии, запись package.py.  
   - Уже есть native bind для platform, arch, os; добавить Rust-реализации для остальных.

4. **Pip: парсинг требований**  
   - Заменить вызов `packaging.requirements` + `Marker` на крейт PEP 508 или свой парсер; оставить fallback для краевых случаев.

5. **Build: pre_build_commands**  
   - Логика уже как в rex; можно вынести общий «rex-движок» (интерпретатор команд с env proxy) в Rust и вызывать из build и env.

### Низкий приоритет / опционально

6. **Loader (package.py)**  
   - Полный отказ от выполнения package.py потребует либо парсера package.py в Rust (только данные), либо перехода на package.yaml. Большой объём работ; для совместимости с Rez выполнение package.py оставить, опционально — «fast path» по AST/статическому анализу.

7. **Build: parse_build_args**  
   - Поддержка произвольного parse_build_args.py — удобство для сложных сборок; порт в Rust — конвенция (например, фиксированный набор флагов) или минимальный парсер.

8. **Config: валидация по схеме Rez**  
   - Опционально: проверка полей конфига по схеме как в rez/config.py после загрузки (Rust).

---

## 4. Детали по компонентам

### 4.1 Config (config.rs) — частично портировано

- **load_rezconfig_source**: выполняется Python-код (bootstrap + пользовательский rezconfig). Результат — модуль; затем вызывается `_extract(globals, ModifyList)` и `json.dumps(result, default=str)` → JSON.  
- **Сделано (вариант B):** добавлен **нативный путь** без Python:
  - Встроенный дефолт: `config/rezconfig_default.json` (packages_path, local/release_packages_path, plugins.pkg_rs.resolver_backend).
  - `load_config_native()`: загрузка дефолта + только YAML/JSON оверрайдов (REZ_CONFIG_FILE, ~/.rezconfig и т.д.), затем `normalize_pkg_rs`, `apply_env_overrides`, `expand_config_vars` (уже в Rust).
  - В `load_config()`: если в цепочке оверрайдов **нет .py** — используется native; иначе или при ошибке native — fallback на Python (`load_rez_config`).
  - Итог: сценарий без rezconfig.py (только дефолт или YAML/JSON) работает **без вызова Python**.

### 4.2 Solver Rez (solver/mod.rs)

- **solve_reqs_rez**: Config(filepaths), _replace_config, ResolvedContext(requirements), resolved_packages.  
- **Порт:** см. документ SOLVER.md в python/rez; алгоритм в solver.py (SolverState, выбор версий, backtracking). Либо адаптер к rez-next-solver с конвертацией наших Package/requirements в их типы.

### 4.3 Loader (loader.rs)

- Выполнение package.py, инъекция pkg (Package, Env, Evar, App, Action), get_package(), извлечение команд через inspect.getsource.  
- **Порт:** альтернатива — парсер package.py (AST) в Rust, извлечение только полей (name, version, requires, commands как строки и т.д.) без выполнения кода. Тогда для «полного» Rez-пакета с произвольным кодом по-прежнему нужен Python.

### 4.4 Rex (rex.rs, build pre_build)

- Один и тот же паттерн: bootstrap (классы _EnvVarProxy, _RexEnv), py.run(source), чтение _evars, применение к Env.  
- **Порт:** интерпретатор подмножества Python (только присваивания env.VAR.set/append/insert) в Rust; либо фиксированный формат команд (например, «одна строка на переменную»).

### 4.5 Bind Python (bind_module.rs)

- **rez.package_bind.bind_package(name, path, no_deps, quiet)**.  
- **Порт:** для каждого имени (python, rez, setuptools, pip) реализовать в Rust: поиск исполняемого файла, определение версии, запись package.py в path (шаблон как у native bind).

### 4.6 Pip requirements (pip.rs)

- **parse_requires_dist_lines**: использование `packaging.requirements.Requirement` и `packaging.markers.Marker`.  
- **Порт:** использовать крейт типа `pep508_rs` или разбор вручную по PEP 508; уже есть `parse_requires_dist_fallback` — расширить его или заменить вызов Python.

---

## 5. Порядок работ (рекомендуемый)

1. **Config** — порт загрузки конфига в Rust (YAML/JSON слои + REZ_* без выполнения rezconfig.py) с опцией fallback на Python.  
2. **Rez solver** — порт алгоритма в Rust как третий бэкенд `Resolver` или замена backend=rez на вызов портированного движка.  
3. **Pip requirements** — парсер PEP 508 в Rust, убрать вызов packaging.  
4. **Bind** — Rust-реализации для python, rez, setuptools, pip.  
5. **Rex/pre_build** — общий rex-движок в Rust (опционально).  
6. **Loader** — опциональный AST/статический парсер package.py (без обязательного порта).

---

## 6. Сводная таблица

| Компонент | Сложность порта | Выигрыш | Зависимость от Rez после порта |
|-----------|------------------|---------|---------------------------------|
| Config | Средняя | Один бинарник без Python для конфига | Нет |
| Solver Rez | Высокая | Нет вызова Rez при resolve | Нет |
| Pip requirements | Низкая | Нет вызова packaging | Нет |
| Bind (python/rez/…) | Средняя | Нет rez.package_bind | Нет |
| Rex / pre_build | Средняя | Единый движок, без py.run для rex | Нет (для нашего кода) |
| Loader | Высокая | Опционально «без выполнения» | Только если оставить выполнение package.py |
| parse_build_args | Средняя | Меньше зависимости от скриптов пакета | Частично |

После порта конфига, солвера Rez, pip requirements и bind — типичный сценарий (scan, resolve, env, build без произвольного parse_build_args) может работать **без дерева python/rez**, если выполнение package.py заменить на парсер/данные (или оставить встроенный Python только для Loader). Полный отказ от Python потребует также порта выполнения package.py (ограниченный интерпретатор или отказ от произвольного кода в package.py).
