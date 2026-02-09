# Статус инструментов pkg (rez-совместимый CLI)

Краткая сводка: что реализовано нативно в Rust; внутренние/вспомогательные команды. **Парность с Rez и оценка:** [PARITY.md](PARITY.md).

---

## Легенда

| Метка | Значение |
|-------|----------|
| **Native** | Полностью в Rust (или Rust + точечный вызов Python API, см. описание). Неподдерживаемые флаги/аргументы — ошибка. |
| **Internal** | Служебная команда (например, для build pipeline). |

---

## Ядро: окружение, сборка, pip

| Команда | Статус | Описание |
|---------|--------|----------|
| **env** | Native | Резолв зависимостей, merge env, выполнение pre/commands/post (rex), stamp, expand. Вывод env или запуск команды после `--`. |
| **build** | Native | Загрузка package.py, варианты, резолв build context, pre_build_commands (Python rex), выбор build system (custom/make/cmake/cargo/python), установка в репозиторий. |
| **build-env** | Internal | Запуск окружения сборки из build.rxt (используется из build pipeline). |
| **pip** | Native | Поиск python/pip, `pip install --target`, разбор метаданных и entry points, копирование в layout репо, генерация package.py. |

---

## Конфиг и контекст

| Команда | Статус | Описание |
|---------|--------|----------|
| **config** | Native | Чтение конфига (rezconfig), вывод путей (--search-list, --source-list), поля, JSON. |
| **context** | Native | Один .rxt (или REZ_RXT_FILE), --print-request, --print-resolve, --show-uris, --print-graph, --write-graph, -f/--format (dict|table|json), --interpret, --which, --no-env. Неизвестный флаг/аргумент → ошибка. |
| **status** | Native | Без аргументов: версия Rez, активный контекст, visible suites. С аргументами → ошибка. |
| **suite** | Native | --list, --create, один DIR. Неизвестный флаг/аргумент → ошибка. |

---

## Bind и совместимость с Rez

| Команда | Статус | Описание |
|---------|--------|----------|
| **bind** | Native | Модульный регистр: **Native** (platform, arch, os — Rust) и **Python** (python, rez, rezgui, setuptools, pip — `rez.package_bind`). Добавить модули: `plugins.pkg_rs.bind_modules_extra` (список имён, стратегия Python). Убрать: `plugins.pkg_rs.bind_modules_remove`. --list / --search по регистру; bind одного имени или --quickstart по всем из регистра. |

---

## Поиск, просмотр, граф зависимостей

| Команда | Статус | Описание |
|---------|--------|----------|
| **search** | Native | Поиск пакетов по паттернам/тегам, --latest, --json. |
| **view** | Native | Детали пакета (reqs, envs, apps), --json. |
| **depends** | Native | Граф зависимостей: list / dot / mermaid, --reverse, --depth. |
| **diff** | Native | Сравнение двух пакетов (файловая разница), опционально внешний difftool. |

---

## Репозиторий и пакеты

| Команда | Статус | Описание |
|---------|--------|----------|
| **cp** | Native | Копирование пакета между репозиториями (с вариантами, без --variant-uri пока). |
| **mv** | Native | Перемещение = копирование + удаление. |
| **rm** | Native | Удаление пакета (--package), семьи (--family), или по --ignored-since. |
| **release** | Native | Загрузка package.py из текущей директории, резолв, сборка, установка в release repo, latest symlink. |
| **pkg-ignore** | Native | Игнорирование пакета в репо (list repos, add/remove ignore). |
| **pkg-cache** | Native | Очистка кэша метаданных пакетов (--clear). |

---

## Тесты и интерпретация

| Команда | Статус | Описание |
|---------|--------|----------|
| **test** | Native | Резолв env, pre_test_commands через rex (merge + solve), запуск записей из секции tests (shell/exec), PASS/FAIL/SKIP, --list, --inplace, фильтры. |
| **interpret** | Native | Минимальный интерпретатор rex-скриптов (файл, формат shell, parent_vars, no_env). |

---

## Плагины и кэши

| Команда | Статус | Описание |
|---------|--------|----------|
| **plugins** | Native | Список пакетов, у которых в plugin_for указан заданный пакет. |
| **memcache** | Native | Интроспекция конфига (memcached_uri); «clear» — сообщение, что memcache в pkg-rs пока не активен. |
| **pkg-cache** | Native | См. выше (репозиторий и пакеты). |

---

## Бандл, бенчмарк, конвертеры

| Команда | Статус | Описание |
|---------|--------|----------|
| **bundle** | Native | Бандл контекста (.rxt) в директорию или zip: копирование пакетов, патчинг ELF/Mach-O (bin-patch), генерация скриптов/контекста. |
| **benchmark** | Native | Бенчмарки резолва/сканирования, --out, --histogram, --compare. |
| **yaml2py** | Native | Конвертация YAML → package.py-подобный вывод. |

---

## Прочее

| Команда | Статус | Описание |
|---------|--------|----------|
| **python** | Native | Запуск скрипта или REPL с pkg в sys.path. |
| **shell** | Native | Интерактивная оболочка с tab-completion. |
| **gui** | Native | GUI (eframe/egui): список пакетов, граф зависимостей, solve, экспорт env. |
| **help** | Native | Вывод справки. |
| **version** | Native | Версия и сборка. |
| **completions** | Native | Генерация shell completions. |
| **selftest** | Native | Проверка конфига и солвера (sanity). |

---

## Сводка по типам

| Тип | Команды |
|-----|---------|
| Native | Все перечисленные выше (включая bind, context, status, suite). Неподдерживаемые опции → ошибка, без вызова Python Rez CLI. |
| Internal | build-env |

Обновлено: все hybrid переведены в native; делегирование в Python Rez (rez_passthrough) удалено.
