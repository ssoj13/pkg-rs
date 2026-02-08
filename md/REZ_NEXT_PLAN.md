# Предварительный план: rez-next — что есть и что можно взять

Дата: 2026-02-08

## Цель

Оценить референсный проект **rez-next** (_ref/rez-next): что в нём реализовано, чего у pkg-rs нет, и что имеет смысл адаптировать или переиспользовать без выбрасывания уже сделанных в pkg-rs кусков (pip, build, репозитории, solver).

---

## 1. Обзор rez-next

**Статус проекта:** экспериментальный, не для продакшена. Python bindings частично отключены (DLL/issues). Лицензия Apache-2.0.

**Структура workspace:**

| Крейт | Назначение | Зависимости |
|-------|------------|-------------|
| rez-next-common | Общие ошибки, конфиг, утилиты | — |
| rez-next-version | Парсинг и сравнение версий | common |
| rez-next-package | Модель пакета, парсинг package.py (AST), валидация | common, version |
| rez-next-solver | Разрешение зависимостей (граф, A* закомментирован) | common, version, package |
| rez-next-repository | Сканирование репозиториев, кэш, async | common |
| rez-next-context | ResolvedContext, .rxt, окружение, shell | common, package, version |
| rez-next-build | Абстракция сборки (артефакты, процесс, источники) | — |
| rez-next-cache | Многоуровневый кэш, предиктивный прогрев | — |

---

## 2. Сравнение: что есть в pkg-rs vs rez-next

### 2.1 Версии

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Модуль | `src/rez_version/` (Version, VersionRange, Token/SubToken) | rez-next-version (StateMachineParser, Version, VersionRange) |
| Парсинг | Свой, Rez-совместимая семантика (алфавитно-цифровые токены) | State machine, заявлено 117x быстрее, токены (Numeric/AlphaNumeric) |
| Сериализация | — | serde (Version as string) |

**Вывод:** У нас уже есть рабочая версионная модель. Имеет смысл при необходимости сравнивать краевые кейсы и бенчмарки; **переносить rez-next-version целиком не обязательно**. Идеи из state machine парсера можно учесть при оптимизации.

### 2.2 Пакет (Package) и загрузка

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Модель | `src/package.rs` — одна большая структура Package | rez-next-package — своя структура Package (Requirement, variants, tools, …) |
| Загрузка package.py | **Выполнение через PyO3** (Loader + get_package()) | **Парсинг AST без выполнения** (RustPython: rustpython_ast, python_ast_parser) |
| Валидация | Разрозненная при загрузке/сборке | PackageValidator, ValidationOptions |
| Сериализация | to_dict/from_dict, YAML в build | YAML, JSON, Python |

**Вывод:** Наш подход (выполнение package.py) даёт полную совместимость с произвольным Python в package.py. **RustPython AST в rez-next** — альтернатива для «только данные без кода»: быстрее и безопаснее для непроверенных репозиториев. Для полного паритета с rez мы пока не отказываемся от выполнения; парсер можно рассмотреть как **опциональный путь** (например, для быстрого скана метаданных или sandbox).

#### Сравнение структуры Package (поля и семантика)

| Поле / аспект | pkg-rs | rez-next |
|---------------|--------|----------|
| Идентичность | `name` (full), `base`, `version: String` | `name` (base), `version: Option<Version>`, `base: Option<String>`; `qualified_name()` = name-version |
| Требования | `reqs: Vec<String>` | `requires: Vec<String>`; отдельно `PackageRequirement` (name, version_spec, weak), `Requirement` + `VersionConstraint` |
| Окружение / приложения | **`envs: Vec<Env>`, `apps: Vec<App>`** (path, args, env_name) | `tools: Vec<String>`, `commands: Option<String>` (rex-строка) |
| Resolved deps | **`deps: Vec<Package>`** (вложенные пакеты) | Нет на Package; контекст держит resolved set |
| Build | `build_system`, `build_command: Option<BuildCommand>`, `build_directory`, `build_args` | `build_command: Option<String>`, `build_system: Option<String>` |
| Команды | `pre_commands`, `commands`, `post_commands`, `pre_test_commands`, `pre_build_commands` | То же + те же имена полей |
| Метаданные | `description`, `authors`, `uuid`, `tags`, `icon` | `description`, `authors`, `uuid`; нет tags/icon |
| Rez-специфичные | `relocatable`, `cachable`, `help`, `tests`, `config`, `variants`, `hashed_variants` | + `timestamp`, `revision`, `changelog`, `release_message`, `previous_version`, `previous_revision`, `vcs`, `format_version`, `preprocess`, `has_plugins`, `plugin_for` |
| Pip | `pip_name`, `from_pip`, `is_pure_python` | Нет |
| Валидация | Разрозненная | `validate()`, `PackageValidator`, `PackageValidationOptions` |
| Типизированные требования | `DepSpec` в dep.rs, парсинг строк | `PackageRequirement::parse()`, `Requirement` + `VersionConstraint` (Exact, Range, …) |

**Вывод по Package:** У rez-next модель **ближе к «сырой» схеме Rez** (commands/tools, один блок команд), без первых классов Env/App. У нас **богаче запуск и окружение** (envs, apps с path/args/env_name). Целиком **менять наш Package на их не стоит**: потеряем envs/apps/deps и привязку к Loader. Имеет смысл **привести к одному интерфейсу/набору полей** (как в 2.4–2.6):  
- Добавить у себя недостающие поля по образцу rez-next: `timestamp`, `revision`, `changelog`, `release_message`, `previous_version`/`previous_revision`, `vcs`, `format_version`, `preprocess` (и при необходимости `base` как явное поле, если сейчас только через name).  
- Ввести тип **требования** в духе rez-next: аналог `PackageRequirement` или `Requirement` + `VersionConstraint` и метод `parse()` для строки; при желании — `validate()` с той же сигнатурой.  
- Сохранить наши `envs`, `apps`, `deps`; для сериализации в .rxt или обмена с кодом, ожидающим «Rez-форму», делать **адаптер/вид** (наш Package → поля как у rez-next Package), а не замену типа.  
Итого: **Package не брать целиком, но выровнять схему полей и API** (те же имена где возможно, типизированные требования, validate) и при необходимости дать фасад под их форму.

### 2.3 Resolver (solver)

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Алгоритм | **PubGrub** + опционально Python rez (ResolvedContext) | DependencySolver, граф; A* и optimized_solver закомментированы |
| Конфликты | Сообщения от PubGrub / от Rez | Модули conflict, resolution упрощены/отключены |
| Фильтры/сортировка | filter.rs, order.rs, из конфига | — |

**Вывод:** У нас уже два бэкенда (pkg, rez). **Заменять наш solver на rez-next-solver нецелесообразно** — у них упрощённое состояние. Можно позаимствовать **идеи формата объяснения конфликтов** (если в rez-next появятся стабильные API).

### 2.4 Репозитории и сканирование

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Абстракция | `PackageRepository` (trait), FilesystemRepository, MemoryRepository | Repository, FileSystemRepository, RepositoryManager, SimpleRepository |
| Сканирование | jwalk (синхронный обход), Loader по каждому package.py | Scanner, HighPerformanceScanner, **async** |
| Кэш | Cache (moka + JSON по путям), в storage | RepositoryCache, cache.rs (TTL, LRU) |
| Использование | Storage.scan_impl → scan_repositories() | RepositoryManager.scan_all_repositories().await |

**Вывод:** У rez-next **интерфейс заметно структурированнее**: trait `Repository` с `initialize()`/`refresh()`, `find_packages(criteria)`, `get_package()`, `get_stats()`, плюс `RepositoryManager`, `RepositoryMetadata`, `PackageSearchCriteria`, `RepositoryStats` (package_count, version_count, last_scan_time и т.д.). У нас — только `location()`, `packages()`, `warnings()`, `packages_for()`/`variants_for()`.  
**Решение (см. раздел 6):** либо взять их API целиком (адаптеры поверх нашей реализации), либо как минимум ввести у себя тот же интерфейс (имена методов и типы аргументов/результатов).

### 2.5 Context и .rxt

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Тип «контекст» | Нет отдельного типа; результат solve — список имён, env строится из Package | **RezResolvedContext** (resolved_packages, requirements, environ, suite_path, timestamp, platform, …) |
| Сериализация .rxt | build.rxt — JSON с переменными окружения (build) | ContextSerializer: JSON .rxt, export to YAML/env/shell |
| Сохранение/загрузка контекста | Только build.rxt для сборки | save_to_file / load_from_file, export_context (Json, Yaml, Env, Shell) |

**Вывод:** У rez-next контекст — **первый класс**: `ResolvedContext` (id, name, requirements, resolved_packages, environment_vars, metadata, created_at, suite, platform, arch, status) плюс в другом файле `RezResolvedContext` с `resolved_packages: Vec<ResolvedPackage>`, `environ`, `suite_path`, `timestamp`, `get_package()`, `get_environ()` и т.д. У нас такого типа нет.  
**Решение (см. раздел 6):** либо взять их структуру/интерфейс целиком (тип + методы get_package, get_environ, сериализация .rxt), либо как минимум ввести у себя тип и API в том же виде (те же поля и имена методов), реализовав логику поверх нашего Package/solver.

### 2.6 Сборка (build)

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Реализация | **Полный пайплайн**: build.rs + build/systems (custom, make, cmake, cargo, python), msvc, варианты, REZ_BUILD_*, install | rez-next-build: абстракции (artifacts, builder, environment, process, sources, systems) |
| Интеграция | Используется в pkg build / rez release | Используется внутри rez-next CLI |

**Вывод:** У rez-next build **явно разнесён по типам**: `BuildManager` (config, active_builds, stats), `BuildConfig`, `BuildRequest` (package, context, source_dir, variant, options, install_path), `BuildProcess` (build_id, request, environment, status, output, errors), `BuildStep` (Preparing, Configuring, Compiling, …), `BuildSystem::detect()`, `BuildEnvironment`, `BuildArtifacts`. У нас — по сути один вызов `build_package(pkg, path, storage, options)` и отчёт.  
**Решение (см. раздел 6):** либо взять их API целиком (BuildManager/BuildProcess/BuildRequest), реализуя внутри нашим пайплайном; либо как минимум выставить у себя тот же интерфейс (те же структуры и методы), чтобы код был структурирован так же.

### 2.7 Кэш (общий)

| Аспект | pkg-rs | rez-next |
|--------|--------|----------|
| Реализация | cache.rs: moka in-memory, JSON persist, mtime invalidation | rez-next-cache: IntelligentCacheManager, L1/L2, predictive preheating, adaptive tuning |
| Назначение | Кэш распарсенных пакетов по путям | Универсальный кэш для компонентов |

**Вывод:** Наш кэш достаточен для текущего сценария. **Идеи из rez-next-cache** (многоуровневость, предиктивный прогрев) — на будущее, если появятся требования по производительности или «умному» кэшу. В предварительный план включаем как **низкий приоритет**.

### 2.8 Pip

В rez-next отдельного крейта «pip» нет; pip-подобная логика не прослеживается в обзоре. У нас **pip реализован в pkg-rs** (pip.rs, команда pip) — ничего брать из rez-next по pip не требуется.

---

## 3. Что можно взять / сделать (приоритеты)

### Высокий приоритет: выравнивание интерфейса с rez-next (2.4, 2.5, 2.6)

1. **Репозитории (2.4)** — либо взять их API целиком, либо как минимум адаптировать под тот же интерфейс:
   - Ввести у себя аналог trait `Repository` с методами: `metadata()`, `initialize()`/`refresh()`, `find_packages(criteria)`, `get_package(name, version)`, `get_package_versions()`, `get_stats()` и т.д.
   - Типы: `RepositoryMetadata`, `PackageSearchCriteria`, `RepositoryStats` (как в сводке выше).
   - `RepositoryManager` (или аналог): добавление/удаление репозиториев, `find_packages()`/`get_package()` по всем, `initialize_all()`/`refresh_all()`.
   - Текущие `FilesystemRepository` и `scan_repositories` реализуют этот интерфейс изнутри (sync-обёртки или постепенный переход на async по необходимости).

2. **Контекст (2.5)** — тот же подход:
   - Ввести тип `ResolvedContext` с полями и методами как у rez-next (id, requirements, resolved_packages, environment_vars, metadata, created_at, suite, platform, arch, status; методы get_package, get_environ, get_tools и т.д.).
   - Реализовать сохранение/загрузку .rxt (JSON), export в YAML/env/shell.
   - Использовать в `pkg context`, `pkg env --save`, suite и при необходимости в build.

3. **Сборка (2.6)** — тот же подход:
   - Ввести у себя `BuildManager`, `BuildConfig`, `BuildRequest`, `BuildProcess`, `BuildStep`, `BuildSystem::detect()`, `BuildEnvironment` (и при необходимости `BuildArtifacts`) с сигнатурами/полями как в rez-next.
   - Текущий `build_package()` и пайплайн (systems, msvc) становятся реализацией за этим API (например, внутри `BuildProcess` или `BuildManager`).

### Средний приоритет

4. **Репозитории:** опционально — идеи из high_performance_scanner (параллель/async, turbo-режим), не ломая новый интерфейс.
5. **Версии:** сверить кейсы с rez/rez-next; при выигрыше рассмотреть перенос только парсера/токенов.

### Низкий приоритет

6. Парсинг package.py без выполнения (RustPython/AST).
7. Идеи кэша (L2, TTL, предиктивный прогрев).
8. Формат сообщений о конфликтах в solver.

---

## 4. Решение по 2.4, 2.5, 2.6: один интерфейс с rez-next

**Тезис:** Репозитории (2.4), контекст (2.5) и сборка (2.6) у rez-next **более структурированы** (отдельные типы, явные методы, единый стиль). Имеет смысл либо **взять их интерфейс целиком** (наши реализации за тем же API), либо **как минимум адаптировать наш код под такой же интерфейс**.

### Варианты

| Вариант | Репозитории | Контекст | Сборка | Package |
|--------|-------------|----------|--------|--------|
| **A. Тот же интерфейс** | Ввести у себя trait и типы как у rez-next (Repository, Manager, Metadata, SearchCriteria, Stats); наша логика — внутри. | Ввести ResolvedContext с полями/методами как у rez-next + .rxt; наш solve/env — внутри. | Ввести BuildManager, BuildProcess, BuildRequest, BuildConfig, BuildEnvironment; наш build_package — внутри. | Выровнять схему полей и API (те же имена, недостающие Rez-поля, типизированные требования, validate); фасад под их форму при необходимости. |
| **B. Взять код и адаптировать** | Скопировать/перенести rez-next-repository, подменив их Package на наш, sync/async по необходимости. | Скопировать/перенести типы и сериализацию из rez-next-context, подставив наш Package. | Скопировать/перенести builder/process/environment/systems, подставив наш Package и конфиг. | Не заменять наш Package целиком (теряем envs/apps/deps); при желании перенести только Requirement/VersionConstraint и PackageValidator. |

Рекомендация: **сначала A** (единый интерфейс без переноса чужого кода). По **Package** — не брать их тип целиком (у нас богаче envs/apps/deps), но **привести к одной схеме**: те же имена полей где возможно, добавить недостающие Rez-поля, ввести типизированные требования и validate(); при обмене с .rxt/rez-next — фасад или сериализация в их форму.

### Сводка интерфейсов rez-next (образец для выравнивания)

**Репозитории (2.4):**
- `Repository` (trait): `metadata()`, `initialize()`, `refresh()`, `find_packages(criteria)`, `get_package(name, version)`, `get_package_versions(name)`, `get_package_variants()`, `package_exists()`, `get_package_names()`, `get_stats()`.
- Типы: `RepositoryMetadata`, `PackageSearchCriteria`, `RepositoryStats` (package_count, version_count, variant_count, size_bytes, last_scan_time, last_scan_duration_ms).
- `RepositoryManager`: `add_repository()`, `remove_repository()`, `get_repository()`, `find_packages()`, `get_package()`, `initialize_all()`, `refresh_all()`.

**Контекст (2.5):**
- `ResolvedContext`: `id`, `name`, `requirements`, `resolved_packages`, `environment_vars`, `metadata`, `created_at`, `suite`, `platform`, `arch`, `status`.
- Методы (по RezResolvedContext): `get_package_names()`, `get_package(name)`, `get_variant()`, `has_package()`, `get_package_version()`, `get_tools()`, `get_environ()`.
- Сериализация: save/load .rxt (JSON), export (YAML, env, shell).

**Сборка (2.6):**
- `BuildManager`: конфиг, активные сборки, статистика.
- `BuildConfig`: build_dir, temp_dir, max_concurrent_builds, timeout, clean_before_build, verbosity, build_env_vars.
- `BuildRequest`: package, context, source_dir, variant, options, install_path.
- `BuildProcess`: build_id, request, environment, status, output, errors; пошаговый жизненный цикл.
- `BuildStep`: Preparing, Configuring, Compiling, Testing, Packaging, Installing, Cleanup.
- `BuildSystem`: enum (CMake, Make, Python, Cargo, Custom и т.д.) с `detect(source_dir)`.
- `BuildEnvironment`: build_dir, install_dir, temp_dir, env_vars, контекст.

**Package (2.2) — выравнивание схемы (не замена):**
- Поля по образцу rez-next: те же имена для requires/build_requires, variants, commands, pre_commands, post_commands, pre_test_commands, pre_build_commands, tests, help, relocatable, cachable, uuid, timestamp, revision, changelog, release_message, vcs, format_version, base, has_plugins, plugin_for, hashed_variants, preprocess, previous_version, previous_revision.
- Типизированные требования: аналог `PackageRequirement` (name, version_spec, weak) и/или `Requirement` + `VersionConstraint`; метод `parse()` для строки.
- Метод `validate()` с той же контрактной семантикой (имя не пустое, формат имени, версия, требования, варианты).
- Наши `envs`, `apps`, `deps` сохраняем; для .rxt или обмена — сериализация/вид в форму rez-next (tools из apps, commands из env/commands).

---

## 5. Что не брать из rez-next

- **Не подключать rez-next как workspace/crate dependency** — разные модели (Package, Version, конфиг), разный стек (tokio, chrono, whoami и т.д.), риски лицензии (Apache-2.0 vs MIT).
- **Не заменять:** наш solver, наш build, наш pip, нашу загрузку package.py по умолчанию (PyO3 execute).
- **Не переносить целиком:** тип Package из rez-next (у них нет envs/apps/deps в нашем виде; мы выравниваем схему полей и API, но не подменяем тип). Не переносить целиком rez-next-context (другой тип контекста и зависимостей).

---

## 6. Следующие шаги

1. **Package (2.2):** выровнять схему — добавить недостающие поля (timestamp, revision, changelog, release_message, vcs, format_version, preprocess, previous_version, previous_revision и т.д.), ввести типизированные требования (PackageRequirement/Requirement + parse), метод validate(); сохранить envs, apps, deps.
2. **Интерфейс репозиториев (2.4):** объявить в pkg-rs trait и типы по сводке выше; реализовать их для текущих FilesystemRepository/MemoryRepository и для `scan_repositories` (sync-обёртки или адаптер через Storage).
3. **Интерфейс контекста (2.5):** зафиксировать схему .rxt; ввести тип ResolvedContext с полями и методами как у rez-next; реализовать сериализацию .rxt и подключить к `pkg context`, `pkg env --save`, suite.
4. **Интерфейс сборки (2.6):** ввести BuildManager, BuildConfig, BuildRequest, BuildProcess, BuildStep, BuildSystem, BuildEnvironment; перенести логику из текущего `build_package()` под этот API.
5. При необходимости — документ «Rez .rxt schema» и таблица соответствия с rez-next/Rez.
6. Репозитории: опционально параллель/async; кэш/версии — по мере необходимости.

После этого можно уточнять план (конкретные файлы rez-next для копирования сигнатур, порядок внедрения Package → Repository → Context → Build).
