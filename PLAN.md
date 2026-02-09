# PLAN — План, статус команд и парность

Единый документ: текущее состояние, статус всех команд, что сделано, что в работе, что прибить. Детали: [md/PLAN.md](md/PLAN.md), [TODO.md](TODO.md), [PARITY.md](PARITY.md).

---

## 1. Текущее состояние (кратко)

- **pkg-rs**: package.py, резолвер (PubGrub/Rez), env + **pre/commands/post** (rex) при `pkg env`, **pre_test + tests** при `pkg test`, build, pip, **все rez-команды нативно** (bind — модульный регистр, extra/remove в конфиге). Неподдерживаемые опции → ошибка.
- **bin-patch** (`crates/bin-patch`): ELF/Mach-O для релоцируемых бандлов.
- **Windows**: санитизация путей python/pip после pip-импорта.

---

## 2. Статус команд (сводка)

**Легенда:** **Native** — реализация в Rust (или Rust + точечный вызов Python API). **Internal** — служебная команда.

### 2.1 Ядро

| Команда | Статус | Описание |
|---------|--------|----------|
| **env** | Native | Резолв, merge env, pre/commands/post (rex), stamp, expand; вывод или запуск после `--`. |
| **build** | Native | package.py, варианты, build context, pre_build_commands, build system (custom/make/cmake/cargo/python), установка. |
| **build-env** | Internal | Окружение сборки из build.rxt. |
| **pip** | Native | Поиск python/pip, pip install --target, метаданные, копирование в репо, package.py. |

### 2.2 Конфиг и контекст

| Команда | Статус | Описание |
|---------|--------|----------|
| **config** | Native | Чтение rezconfig, пути, поля, JSON. |
| **context** | Native | .rxt (REZ_RXT_FILE), --print-request/resolve, --format, --which и др.; неизвестное → ошибка. |
| **status** | Native | Без аргументов: версия, контекст, suites; с аргументами → ошибка. |
| **suite** | Native | --list, --create, DIR; неизвестное → ошибка. |

### 2.3 Bind

| Команда | Статус | Описание |
|---------|--------|----------|
| **bind** | Native | Регистр: Native (platform, arch, os, python, rez, setuptools, pip) + config extra. Конфиг: `bind_modules_extra` / `bind_modules_remove`. Clap: -l, -s, --quickstart, -r, -i, &lt;name&gt;. |

### 2.4 Поиск, граф, репозиторий

| Команда | Статус |
|---------|--------|
| **search, view, depends, diff** | Native |
| **cp, mv, rm, release, pkg-ignore, pkg-cache** | Native |

### 2.5 Тесты и прочее

| Команда | Статус |
|---------|--------|
| **test** | Native (pre_test rex, tests, --inplace) |
| **interpret** | Native |
| **plugins, memcache** | Native |
| **bundle, benchmark, yaml2py** | Native |
| **python, shell, gui, help, version, completions, selftest** | Native |

**Итог:** все команды Native или Internal; passthrough удалён.

---

## 3. Сделано (отмечено ✅)

| # | Задача | Отметка |
|---|--------|--------|
| 1 | Выполнение pre/commands/post при `pkg env` (rex) | ✅ |
| 2 | Тесты пакетов: pre_test_commands (rex) + секция tests, .rxt inplace | ✅ |
| 3 | Passthrough → native (bind, context, status, suite) | ✅ |
| 4 | Bind: модульный регистр, extra/remove в конфиге | ✅ |

---

## 4. В работе / надо сделать

| # | Задача | Приоритет | Пометка |
|---|--------|-----------|--------|
| 5 | Пути python/pip на Windows | по багам | Уже санитизация; при багах — не писать абсолютные пути в конфиг/репо. |
| 6 | Документация | высокий | ✅ USERGUIDE/AGENTS обновлены; примеры конфига (bind, resolver, filter/orderers); rex, test, shell, suite, caches. |
| 7 | Интеграционные тесты | высокий | ✅ rex (pre_commands invoke), test section (pre_test_commands loaded). |
| 8 | Filters/orderers в резолвере | средний | ✅ Уже подключены (backend=pkg + plugins); документировано. |
| 9 | Shell plugins, suite visibility | средний | ✅ Форматы env и правила suite описаны в USERGUIDE/AGENTS. |
| 10 | Кэши (resolve, memcache, package) | низкий | ✅ Package cache реализован; resolve/memcache задокументированы в AGENTS/USERGUIDE. |

---

## 5. Python каталог — когда нужен

**Встроенные bind-модули** (platform, arch, os, python, rez, setuptools, pip) реализованы в Rust; для них **python/ не нужен**.

**python/ обязателен только если** в конфиге задан `bind_modules_extra`: тогда при bind такого имени вызывается `rez.package_bind` (Python). В остальных случаях bind не трогает Python.

**Loader и pkg python:** вызов `ensure_rez_on_sys_path` сделан мягким (ошибка игнорируется). Без каталога python/ загрузка package.py и `pkg python` работают; если в package.py есть `import rez`, пользователь может добавить путь к rez в PYTHONPATH сам.

| Что | Когда нужен python/ |
|-----|----------------------|
| Загрузка package.py, `pkg python` | Нет (rez в path опционально). |
| `pkg bind` встроенные модули | Нет. |
| `pkg bind <имя из bind_modules_extra>` | Да (нужны python/rez/, python/rezplugins/). |
| CMake build system (Rez-шаблоны) | Если есть python/rezplugins/build_system/cmake_files. |

**Вывод: каталог python/ можно не класть** при сборке/распространении, если не используете bind_modules_extra и не нужны Rez CMake-модули. Для bind extra по-прежнему нужны `python/rez/` и `python/rezplugins/`.

---

## 6. Порядок выполнения

1. ~~п.1–4~~ — сделано.
2. ~~Консолидация~~ — PLAN + STATUS в один файл, STATUS.md удалён, ссылки обновлены. ✅
3. ~~Python: удалить `python/rez/tests/`~~ — удалено (тесты Rez не используются из pkg-rs). ✅
4. **Дальше:** п.6 (документация), п.7 (интеграционные тесты), п.8–10.

---

## 7. Ссылки

- [PARITY.md](PARITY.md) — парность с Rez, оценка (~85%).
- [md/PORT_TO_RUST.md](md/PORT_TO_RUST.md) — полный обзор границ Python и план портирования на Rust (config, solver, bind, pip, rex, loader).
- [md/PLAN.md](md/PLAN.md) — интеграционный план (build, pip, плагины).
- [TODO.md](TODO.md) — Rez parity roadmap.
- [AGENTS.md](AGENTS.md) — архитектура, потоки данных.
