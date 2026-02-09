# Портирование на Rust — полный обзор

Дата: 2026-02-08

Документ перечисляет **все** места, где pkg-rs вызывает Python или зависит от кода Rez в `python/rez`, и даёт план портирования на Rust с приоритетами.

---

## 1. Карта границ Python

| Компонент | Файл | Что делает | Зависимость от Rez |
|-----------|------|------------|--------------------|
| **Config** | `config.rs` | Загрузка конфига: выполнение rezconfig.py (и override файлов), извлечение переменных в JSON (ModifyList, DelayLoad, _obj_dict, json.dumps). Применение REZ_* и REZ_*_JSON. | Выполняет **любой** rezconfig.py (Python); нужны классы ModifyList, DelayLoad и хелпер `_extract` для конвертации модуля в dict→JSON. |
| **Solver (Rez backend)** | `solver/mod.rs` | При `resolver_backend=rez`: используется тот же PubGrub и пакеты из Storage (без Python). | **Портировано:** rez.config и rez.resolved_context не вызываются. |
| **Loader** | `loader.rs` | Выполнение package.py: globals (pkg, Package, Env, …), py.run(code), get_package(), извлечение pre_commands и т.д. | Нужен **интерпретатор Python** для выполнения произвольного package.py. |
| **Rex (env)** | `rex.rs` | Выполнение pre_commands/commands/post_commands: bootstrap _RexEnv, py.run(source), сбор _evars, применение к Env. | Только **стандартная библиотека** (os, sys); логика rex — наш код в строке. |
| **Bind** | `bind_module.rs`, `bind/` | Встроенные модули (platform, arch, os, python, rez, setuptools, pip) — нативный Rust, один файл на пакет. Имена из config extra — fallback на rez.package_bind. | **Портировано** для встроенных; rez.package_bind только для доп. имён из конфига. |
| **Build: parse_build_args** | `build.rs` | Выполнение parse_build_args.py из пакета: argparse, parser.add_argument из кода, parse_known_args, vars(), shlex.quote → env REZ_BUILD_*. | **Любой** parse_build_args.py пакета (Python); stdlib argparse, shlex. |
| **Build: pre_build_commands** | `build.rs` | Rex-подобное выполнение pre_build_commands: bootstrap _BuildEnv, this/build/install, py.run(source), вызов pre_build_commands(), чтение _evars. | Только **stdlib**; семантика как в rex.rs. |
| **Pip: requirements** | `pip.rs` | Парсинг Requires-Dist: packaging.requirements.Requirement, packaging.markers.Marker.evaluate → (name, specifier, marker). | Внешняя библиотека **packaging** (PEP 508); есть fallback без Python. |
| **Storage/Repo** | `storage.rs`, `repo/*.rs` | Инициализация Python (для Loader при скане). | Только **Python::initialize**; логика скана — Rust. |
| **Package (help)** | `package.rs` | Optional: inspect.getsource(help_obj) для секции help. | stdlib **inspect**. |
| **CLI: pkg py** | `pkg/python.rs` | REPL или запуск скрипта: sys.path, pkg module, run(code). | Нужен **интерпретатор** для интерактива и скриптов. |

---

## 2. Зависимости от дерева python/rez

Что реально импортируется из `python/rez`:

| Импорт | Где используется | Статус |
|--------|------------------|--------|
| **rez.config** | Раньше: Solver (Rez backend) | **Не используется:** backend=rez теперь использует наш конфиг и PubGrub. |
| **rez.resolved_context** | Раньше: Solver (Rez backend) | **Не используется:** разрешение делается в Rust (PubGrub). |
| **rez.package_bind** | Bind: только для имён из `plugins.pkg_rs.bind_modules_extra` | Встроенные модули (platform, arch, os, python, rez, setuptools, pip) портированы в Rust; rez.package_bind — только для дополнительных имён из конфига. |
| **rez.config (схема)** | Не вызывается напрямую | Опционально: валидация по схеме в Rust. |

Остальное (loader, rex, build pre_build, config load) либо выполняет **произвольный пользовательский код** (package.py, rezconfig.py, parse_build_args.py), либо наш собственный мини-код (rex bootstrap, config bootstrap). Портирование «всего» значит:

- **Конфиг** — портировано: нативный путь (YAML/JSON + REZ_*), fallback на rezconfig.py при наличии .py в цепочке.
- **Solver Rez** — портировано: backend=rez использует PubGrub и пакеты из Storage.
- **Bind** — портировано для встроенных модулей (один файл на пакет в `src/bind/`); setuptools/pip пока только заглушки версии (без окружения).
- **Loader** — выполнение package.py в Python; опционально в будущем — парсер package.py (AST) в Rust.
- **Rex** — наш код в строке; опционально — порт интерпретатора команд в Rust.
- **Build parse_build_args** — Python для произвольных скриптов пакета; опционально — минимальный парсер в Rust.
- **Pip requirements** — fallback без Python есть; опционально — крейт PEP 508.

---

## 3. Приоритеты портирования

### Высокий приоритет (убираем зависимость от Rez для типичного сценария)

1. **Config загрузка** — **сделано:** нативный путь (дефолт + YAML/JSON + REZ_*), при отсутствии .py в цепочке Python не вызывается.

2. **Rez solver** — **сделано:** backend=rez использует PubGrub и пакеты из Storage, без вызова rez.config и rez.resolved_context.

3. **Bind (встроенные модули)** — **сделано:** platform, arch, os, python, rez, setuptools, pip в Rust (`src/bind/`, один файл на пакет). setuptools/pip копируют модуль в пакет и настраивают PYTHONPATH (и для pip — PATH, exe в bin), как в Rez.

### Средний приоритет

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

### 4.2 Solver Rez (solver/mod.rs) — портировано

- **Было:** solve_reqs_rez вызывал rez.config.Config, _replace_config, ResolvedContext(requirements).  
- **Сделано:** backend=rez использует тот же PubGrub и те же пакеты из Storage (solve_reqs_pubgrub). Вызовы Python (rez.config, rez.resolved_context) удалены. Фильтры и ордереры из конфига по-прежнему применяются. Для строгой совместимости с алгоритмом Rez (solver.py) можно добавить отдельный бэкенд или опцию позже.

### 4.3 Loader (loader.rs)

- Выполнение package.py, инъекция pkg (Package, Env, Evar, App, Action), get_package(), извлечение команд через inspect.getsource.  
- **Порт:** альтернатива — парсер package.py (AST) в Rust, извлечение только полей (name, version, requires, commands как строки и т.д.) без выполнения кода. Тогда для «полного» Rez-пакета с произвольным кодом по-прежнему нужен Python.

### 4.4 Rex (rex.rs, build pre_build)

- Один и тот же паттерн: bootstrap (классы _EnvVarProxy, _RexEnv), py.run(source), чтение _evars, применение к Env.  
- **Порт:** интерпретатор подмножества Python (только присваивания env.VAR.set/append/insert) в Rust; либо фиксированный формат команд (например, «одна строка на переменную»).

### 4.5 Bind (bind_module.rs, bind/) — портировано

- Встроенные модули: `src/bind/` — один файл на пакет (platform, arch, os, python, rez, setuptools, pip). Трейт `BindHandler`, регистр в `builtin_handlers()`. Имена из `plugins.pkg_rs.bind_modules_extra` — fallback на rez.package_bind.
- setuptools/pip: как в Rez — копируем установленный модуль в пакет (root/python/…), прописываем PYTHONPATH и для pip ещё PATH и копию exe в bin.

### 4.6 Pip requirements (pip.rs)

- **parse_requires_dist_lines**: использование `packaging.requirements.Requirement` и `packaging.markers.Marker`.  
- **Порт:** использовать крейт типа `pep508_rs` или разбор вручную по PEP 508; уже есть `parse_requires_dist_fallback` — расширить его или заменить вызов Python.

---

## 5. Порядок работ (рекомендуемый)

1. **Config** — сделано.  
2. **Rez solver** — сделано (backend=rez → PubGrub).  
3. **Bind** — сделано для встроенных модулей.  
4. **Pip requirements** — парсер PEP 508 в Rust (опционально); fallback есть.  
5. **Rex/pre_build** — общий rex-движок в Rust (опционально).  
6. **Loader** — опциональный AST/парсер package.py.

---

## 6. Сводная таблица

| Компонент | Сложность порта | Выигрыш | Зависимость от Rez после порта |
|-----------|------------------|---------|---------------------------------|
| Config | Сделано | Нативный путь без Python при отсутствии .py | Нет |
| Solver Rez | Сделано | backend=rez без rez.config/resolved_context | Нет |
| Bind (встроенные) | Сделано | Нет rez.package_bind для platform, arch, os, python, rez, setuptools, pip | Нет (для extra — есть fallback) |
| Pip requirements | Низкая | Fallback есть; опционально крейт PEP 508 | Частично |
| Rex / pre_build | Средняя | Единый движок, без py.run для rex | Нет (для нашего кода) |
| Loader | Высокая | Опционально «без выполнения» | Только если оставить выполнение package.py |
| parse_build_args | Средняя | Меньше зависимости от скриптов пакета | Частично |

После порта конфига, солвера Rez и bind типичный сценарий (scan, resolve, env, bind встроенных модулей) работает **без дерева python/rez**. Для выполнения package.py по-прежнему нужен Python (Loader). Имена из `bind_modules_extra` при bind используют fallback rez.package_bind.
