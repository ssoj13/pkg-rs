# PLAN — Текущий фокус и следующие шаги

Краткий план «что дальше»; детальные планы по билду/pip и Rez parity — в [md/PLAN.md](md/PLAN.md) и [TODO.md](TODO.md). **Сводка парности и оценка:** [PARITY.md](PARITY.md).

---

## Текущее состояние (кратко)

- **pkg-rs**: сканирование package.py, резолвер (PubGrub/Rez), env/evars, **pre/commands/post при pkg env** (rex), **pre_test + tests при pkg test**, запуск приложений, build, pip import, GUI, **все rez-команды нативно** (bind — модульный регистр с extra/remove в конфиге); неподдерживаемые опции → ошибка.
- **bin-patch** (`crates/bin-patch`): ELF/Mach-O in-place патчинг для релоцируемых бандлов.
- **Windows**: санитизация путей к python/pip после pip-импорта.

---

## Приоритетные задачи (по порядку)

### 1. Выполнение команд пакета при `pkg env` (Rez parity) — сделано

**Было:** при `pkg env` применялись только env/evars из package.py. Поля `pre_commands`, `commands`, `post_commands` не выполнялись.

**Сделано:**
- Добавлен модуль **`src/rex.rs`**: bootstrap Python (proxy env с set/append/insert), `apply_package_commands(env, package, command_source, root_path)` — выполняет rex-подобный код и мержит записанные _evars в Env; `packages_in_rex_order(pkg)` — порядок deps затем корневой пакет; `package_root_from_source(package_source)` — ROOT из пути к package.py.
- В **`src/pkg/commands/env.rs`** после получения env из `pkg._env()`: для каждой фазы (pre_commands, commands, post_commands) и каждого пакета в порядке deps → root вызывается `apply_package_commands`; при ошибке выполнения — вывод в stderr и ExitCode::FAILURE.
- Порядок как в Rez: все pre_commands (deps + self), затем все commands, затем все post_commands. Если у пакета нет команд — этап пропускается.

**Файлы:** `src/rex.rs`, `src/lib.rs` (pub mod rex), `src/pkg/commands/env.rs`.

---

### 2. Тесты пакетов (pre_test_commands + тестовые записи) — сделано

**Цель:** выполнение `pre_test_commands` и записей из секции `tests` с отчётом (Rez parity для тестирования пакетов).

**Сделано:**
- Команда `pkg test` уже была (`src/pkg/commands/rez_test.rs`): парсинг секции `tests` (Rez-схема: command, requires, run_on, on_variants), фильтры по имени и run_on, запуск shell/exec, отчёт PASS/FAIL/SKIP.
- **pre_test_commands** теперь выполняются через **rex** (как в п.1): `apply_package_commands` + мерж в env, затем повторный `solve_impl` перед запуском каждого теста. Скрипт может менять окружение (env.PATH.append и т.д.).
- **--inplace**: загрузка контекста из `REZ_RXT_FILE` поддерживает и наш формат .rxt (resolved_packages[].base, .version), и формат Rez (resolved_packages[].variables.name/.version).

---

### 3. Passthrough → native — сделано

**Было:** bind/context/status/suite при неподдерживаемых флагах делегировали в Python rez (passthrough).

**Сделано:** все четыре команды полностью нативные. bind: --list/--search выводят список/поиск по имени; неизвестный пакет или флаги → ошибка. context/status/suite: неизвестные флаги/аргументы → ошибка. Модуль `rez_passthrough` удалён.

---

### 4. Пути к python/pip на Windows

**Цель:** нигде не записывать и не «пробивать» абсолютные пути к python/pip в конфиги/скрипты под установку (python-environments, modules, packages), чтобы не ломать переносимость.

**Уже сделано:** санитизация в скриптах после pip-импорта.

**Дополнительно:** при появлении багов — искать места, где пути могут попадать в конфиг или в файлы репозитория, и править (не писать абсолютные пути или релативизировать при бандле).

---

### 5. Документация и тесты

- Обновить USERGUIDE/AGENTS при изменении поведения (например после п.1).
- Добавить интеграционные/регрессионные тесты на выполнение команд пакета и на тесты пакетов (после п.1–2).
- При необходимости — краткие примеры bundle/pip/rez config в README или docs.

---

## Парность и оценка

Сводка «что сделано / что надо» и оценка по категориям — в **[PARITY.md](PARITY.md)**. Кратко: конфиг, package schema, env+rex, тесты пакетов, все CLI команды (включая bind с модулями) — готовы; типичный сценарий ~85% parity; в приоритете — интеграционные тесты, документация, затем filters/orderers в резолвере, shell plugins, кэши.

---

## Ссылки

- [PARITY.md](PARITY.md) — парность с Rez, что сделано, что осталось, оценка.
- [STATUS.md](STATUS.md) — статус каждой команды.
- [md/PLAN.md](md/PLAN.md) — интеграционный план (build, pip, плагины, фазы).
- [TODO.md](TODO.md) — Rez parity roadmap, конфиг, CLI, репозитории, резолвер.
- [AGENTS.md](AGENTS.md) — архитектура, потоки данных, места в коде.

---

## Порядок выполнения

1. ~~п.1 (выполнение pre/commands/post при `pkg env`)~~ — сделано.
2. ~~п.2 (тесты пакетов: pre_test_commands + rex, .rxt inplace)~~ — сделано.
3. ~~п.3 (passthrough → native)~~ — сделано.
4. **Дальше:** п.4–5 по необходимости.
