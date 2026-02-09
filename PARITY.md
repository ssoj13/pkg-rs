# Парность с Rez и статус

Сводка: что уже сделано, что осталось, ориентировочная оценка.

---

## Rez vs pkg-rs — сводка

### CLI: команда за командой

| Rez | pkg-rs | Статус |
|-----|--------|--------|
| rez-env | pkg env | ✅ Полный: resolve, rex (pre/commands/post), stamp, запуск команды после -- |
| rez-build | pkg build | ✅ Полный: build systems (custom/make/cmake/cargo/python), pre_build_commands, install, --build-args и после -- |
| rez-pip | pkg pip | ✅ Полный: install, -i/-r/-p, --python-version, **все аргументы после пакета → pip install** (как rez-pip -e) |
| rez-bind | pkg bind | ✅ Полный: --list, --search, --quickstart, -r, --no-deps, -i, пакет; clap, справка; native модули (platform, arch, os, python, rez, setuptools, pip) |
| rez-config | pkg config | ✅ Чтение конфига, --json, --search-list, --source-list, поле |
| rez-context | pkg context | ✅ Создание/загрузка контекста, .rxt |
| rez-status | pkg status | ✅ Статус контекста |
| rez-suite | pkg suite | ✅ Список/создание сьютов |
| rez-search | pkg search | ✅ Поиск пакетов, -t, -L, --json |
| rez-view | pkg view | ✅ Просмотр пакета |
| rez-depends | pkg depends | ✅ Граф зависимостей |
| rez-diff | pkg diff | ✅ Сравнение контекстов |
| rez-test | pkg test | ✅ pre_test_commands, tests, --list, --inplace |
| rez-release | pkg release | ✅ Release в репо |
| rez-cp | pkg cp | ✅ Копирование пакетов |
| rez-mv / rez-rm | pkg mv, pkg rm | ✅ Перемещение/удаление |
| rez-pkg-cache | pkg pkg-cache | ✅ Очистка/статистика кэша пакетов |
| rez-pkg-ignore | pkg pkg-ignore | ✅ Игнор по паттернам |
| rez-interpret | pkg interpret | ✅ Выполнение rex-кода |
| rez-plugins | pkg plugins | ✅ Список плагинов (заглушка) |
| rez-memcache | pkg memcache | ✅ Заглушка (clear/stats) |
| rez-yaml2py | pkg yaml2py | ✅ Конвертация yaml → package.py |
| rez-bundle | pkg bundle | ✅ Контекст в dir/zip, bin-patch |
| rez-benchmark | pkg benchmark | ✅ Бенчмарк резолва |
| rez-python | pkg python | ✅ REPL / запуск скрипта |
| rez-help | pkg help | ✅ Справка |
| rez-selftest | pkg selftest | ✅ Самотесты |
| rez-gui | pkg gui | ✅ GUI (граф, solve, экспорт) |
| shell (rez) | pkg shell | ✅ Интерактивная оболочка |

Все команды нативные (без вызова Python Rez CLI). Неподдерживаемые флаги → ошибка.

### Ядро: что без Python, что с Python

| Компонент | Rez | pkg-rs | Зависимость от Python |
|-----------|-----|--------|------------------------|
| **Конфиг** | rezconfig.py + override | Нативный путь (YAML/JSON + REZ_*) без .py; при .py в цепочке — fallback на Python | Без Python при конфиге без .py |
| **Резолвер** | Алгоритм Rez или плагины | Оба бэкенда (pkg, rez) = **PubGrub** + пакеты из Storage; фильтры/ордереры из конфига | **Нет** (rez.config / resolved_context не вызываются) |
| **Loader (package.py)** | Выполнение package.py | Выполнение package.py (PyO3) | **Да** — нужен интерпретатор |
| **Bind** | rez.package_bind или модули | Встроенные модули (platform, arch, os, python, rez, setuptools, pip) — **нативно**; имена из конфига — fallback rez.package_bind | Только для имён из bind_modules_extra |
| **Rex (commands)** | rex в контексте | pre/commands/post через наш rex (py.run bootstrap) | Да (выполнение кода) |
| **Build** | pre_build, build system | Аналогично; parse_build_args — Python при наличии скрипта | По необходимости |
| **Pip** | rez-pip, packaging | pip install --target, dist-info, entry points; конфиг pip_extra_args, remaps | Для сложных требований — fallback packaging |

### Что не сделано или частично

- **Резолвер:** timestamp/patch locks, часть опций блокировок версий.
- **Shell plugins:** формат env под bash/cmd/pwsh (цитирование, alias) — не как в Rez.
- **Кэши:** resolve cache, memcache — только заглушки; pkg-cache есть.
- **Build:** централизованный flow, build-env скрипты как в Rez; часть REZ_BUILD_*.
- **Context/Suite:** полная семантика visibility (tool/suite visibility).
- **Репозиторий:** trait + memory backend для тестов.
- **Package:** filters/orderers из конфига — подключены к резолверу; расширенные варианты — по мере надобности.

---

## Что сделано

### Ядро и окружение
| Область | Статус | Детали |
|--------|--------|--------|
| Конфиг | ✅ | Нативный путь (дефолт + YAML/JSON + REZ_*) без Python; при .py — fallback. package config section для build/release. |
| Package schema | ✅ | Поля name, version, reqs, deps, envs, apps, variants, pre_build/pre/post_commands, pre_test_commands, tests, package_source, plugin_for и др. |
| Резолвер | ✅ | PubGrub для обоих бэкендов (pkg и rez); фильтры/ордереры из конфига. **Без вызова Python.** |
| Env | ✅ | Merge, compress, solve (токены), stamp (PKG_*), commit; pre_commands / commands / post_commands при `pkg env` через rex (src/rex.rs). |
| Тесты пакетов | ✅ | pre_test_commands через rex; секция tests (command, requires, run_on); --list, --inplace; наш и Rez .rxt для inplace. |

### CLI — все команды нативные
| Группа | Команды |
|--------|---------|
| Env/build/pip | env, build, build-env, pip (полный синтаксис rez-pip: аргументы после пакета → pip) |
| Конфиг и контекст | config, context, status, suite |
| Bind | bind на clap (BindArgs): -l/-s, --quickstart, -r, --no-deps, -i; native модули (platform, arch, os, python, rez, setuptools, pip) |
| Поиск и граф | search, view, depends, diff |
| Репозиторий | cp, mv, rm, release, pkg-ignore, pkg-cache |
| Тесты и rex | test, interpret |
| Остальное | plugins, memcache, bundle, benchmark, yaml2py, python, shell, gui, help, version, completions, selftest |

Делегирования в Python Rez CLI нет: неподдерживаемые флаги → ошибка.

### Build и Pip
| Область | Статус |
|--------|--------|
| Build | ✅ Загрузка package.py, варианты, build context, pre_build_commands, выбор build system (custom/make/cmake/cargo/python), установка в репо; --build-args/--child-build-args с `-`; MSVC env (Windows). |
| Pip | ✅ Синтаксис rez-pip: -i, -r, -p, --python-version, пакет; всё после пакета пробрасывается в pip install. Import в репо, entry points, санитизация путей (Windows). |
| Bundle | ✅ Контекст в dir/zip, bin-patch (ELF/Mach-O). |

### Прочее
- .rxt сериализация (наш формат; context/status/suite/rez_test умеют и Rez-формат где нужно).
- GUI (граф, solve, экспорт env).
- bin-patch (crates/bin-patch): ELF/Mach-O без patchelf/install_name_tool.

---

## Что ещё надо (по приоритету)

### Высокий приоритет (важно для повседневного parity)
| Задача | Оценка | Комментарий |
|--------|--------|-------------|
| Интеграционные тесты | 2–3 дн | pkg env (rex), pkg test, pkg build, pkg pip на тестовом репо; регрессии. |
| Документация | 1 дн | USERGUIDE/AGENTS синхрон с текущим поведением; примеры конфига (bind_modules, resolver_backend). |
| Пути python/pip (Windows) | по багам | Уже есть санитизация; при появлении багов — проверять запись абсолютных путей в конфиг/репо. |

### Средний приоритет (улучшение parity)
| Задача | Оценка | Комментарий |
|--------|--------|-------------|
| Package filters / orderers | 2–4 дн | Резолвер: фильтры и упорядочивание пакетов по конфигу Rez. |
| Timestamp / patch locks | 1–2 дн | Блокировки версий при резолве. |
| package.yaml I/O | 1–2 дн | Чтение/запись package.yaml наравне с package.py. |
| Валидация конфига по схеме Rez | 1 дн | Проверка ключей и типов при загрузке. |
| Shell plugins | 2–3 дн | Формат env под bash/cmd/pwsh, цитирование, alias. |
| Suite: tool/suite visibility | 1–2 дн | Правила видимости инструментов и сьютов. |

### Низкий приоритет (расширения)
| Задача | Оценка | Комментарий |
|--------|--------|-------------|
| Resolve caching / memcache | 2–3 дн | Кэш резолва, memcached. |
| Package file / listdir cache | 1–2 дн | Кэширование файлов и listdir в репо. |
| Репозиторий: trait + memory backend | 2 дн | Абстракция репо, memory backend для тестов. |
| Build: central flow, build env scripts | 2–4 дн | Централизованная сборка, скрипты build-env как в Rez. |
| Pip: полный PEP440, distlib, remap | 1–2 дн | Уточнение конвертации версий и правил копирования. |
| Alias / forwarders (rezolve) | 0.5 дн | Поведение алиасов и перенаправление вызовов. |

---

## Оценка парности

| Категория | Готово | Оценка |
|-----------|--------|--------|
| Конфиг | Загрузка + override | ~85% |
| Package schema | Поля + команды в runtime | ~90% |
| Резолвер | Оба бэкенда = PubGrub, filters/orderers | ~85% (нет timestamp/patch locks) |
| Env + rex | pre/commands/post + stamp/solve | ~95% |
| CLI команды | Все нативные, pip/bind полный синтаксис | 100% по покрытию, parity флагов |
| Build | Локальный build, системы сборки | ~75% (нет central, часть REZ_BUILD_*) |
| Pip | Полный синтаксис rez-pip, import в репо | ~85% |
| Bind | clap, native модули (7 шт.), справка | ~90% |
| Context/Suite | .rxt, status, suite list/create | ~70% (нет полной семантики visibility) |
| Тесты пакетов | pre_test + tests, inplace | ~90% |
| Caching | pkg-cache (clear), нет resolve/memcache | ~30% |

**Итог:** для типичного сценария (package.py, resolve, env, pre/commands/post, test, build, pip, bind, context, suite) парность **~85%**. Не хватает в основном: filters/orderers/locks в резолвере, shell plugins, кэши резолва, центральный build и мелкие детали конфига/схемы.

---

## Доделать (приоритет и шаги)

Что брать в работу первым и как именно.

### Высокий приоритет

| # | Задача | Что делать |
|---|--------|------------|
| 1 | **Интеграционные тесты CLI** | Тесты через `std::process::Command`: запуск `pkg search`, `pkg env <pkg>` на тестовом репо (tempdir). Реализовано: test_cli_search_lists_packages, test_cli_env_prints_env. |
| 2 | **Документация** | USERGUIDE/AGENTS в синхрон с кодом; в AGENTS — таблица команд и ссылки на clap; примеры конфига (resolver_backend, package_filter, package_orderers, bind_modules). |
| 3 | **Пути python/pip (Windows)** | По багам: при репортах — проверять запись путей в конфиг/репо, санитизация уже есть. |

### Средний приоритет

| # | Задача | Что делать |
|---|--------|------------|
| 4 | **Timestamp / patch locks в резолвере** | Сейчас: package_filter с правилом `timestamp` и orderer `soft_timestamp` есть. Доделать: семантика Rez «patch lock» (жёсткая привязка к патч-версии) в провайдере/индексе при резолве. |
| 5 | **Валидация конфига** | При загрузке конфига проверять ключи и типы по схеме Rez; единый метод `validate(config)` и понятные ошибки. |
| 6 | **package.yaml I/O** | Чтение package.yaml (как package.py), запись при release/build где Rez пишет yaml. |
| 7 | **Shell plugins** | Формат вывода env под bash/cmd/pwsh (цитирование, alias); плагин по конфигу. |
| 8 | **Suite: tool/suite visibility** | Правила видимости инструментов и сьютов как в Rez. |

### Низкий приоритет

| # | Задача | Что делать |
|---|--------|------------|
| 9 | **Resolve cache / memcache** | Кэш результата резолва; опция memcached, инвалидация. |
| 10 | **Build: central flow, build-env** | Централизованная сборка, скрипты build-env как в Rez. |
| 11 | **Репозиторий: trait + memory** | Абстракция репо, memory backend для тестов. |

Рекомендуемый порядок: **1 → 2** (сначала тесты CLI, потом освежить доки).

---

## Ссылки

- [PLAN.md](PLAN.md) — приоритеты и порядок работ.
- [PLAN.md](PLAN.md) — план, статус команд (§2), сделано/надо (§3–4), Python каталог (§5).
- [TODO.md](TODO.md) — детальный Rez parity roadmap.
