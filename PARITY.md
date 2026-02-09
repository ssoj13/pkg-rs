# Парность с Rez и статус

Сводка: что уже сделано, что осталось, ориентировочная оценка.

---

## Что сделано

### Ядро и окружение
| Область | Статус | Детали |
|--------|--------|--------|
| Конфиг | ✅ | Загрузка rezconfig.py, REZ_CONFIG_FILE, ~/.rezconfig, REZ_* / REZ_*_JSON; package config section для build/release; rezplugins в дереве. |
| Package schema | ✅ | Поля name, version, reqs, deps, envs, apps, variants, pre_build/pre/post_commands, pre_test_commands, tests, package_source, plugin_for и др. |
| Резолвер | ✅ | PubGrub (нативно) + Rez backend (Python); `plugins.pkg_rs.resolver_backend`. |
| Env | ✅ | Merge, compress, solve (токены), stamp (PKG_*), commit; pre_commands / commands / post_commands при `pkg env` через rex (src/rex.rs). |
| Тесты пакетов | ✅ | pre_test_commands через rex; секция tests (command, requires, run_on); --list, --inplace; наш и Rez .rxt для inplace. |

### CLI — все команды нативные
| Группа | Команды |
|--------|---------|
| Env/build/pip | env, build, build-env, pip |
| Конфиг и контекст | config, context, status, suite |
| Bind | bind (модульный регистр: native platform/arch/os + python; extra/remove через конфиг) |
| Поиск и граф | search, view, depends, diff |
| Репозиторий | cp, mv, rm, release, pkg-ignore, pkg-cache |
| Тесты и rex | test, interpret |
| Остальное | plugins, memcache, bundle, benchmark, yaml2py, python, shell, gui, help, version, completions, selftest |

Делегирования в Python Rez CLI нет: неподдерживаемые флаги → ошибка.

### Build и Pip
| Область | Статус |
|--------|--------|
| Build | ✅ Загрузка package.py, варианты, build context, pre_build_commands, выбор build system (custom/make/cmake/cargo/python), установка в репо; --build-args/--child-build-args с `-`; MSVC env (Windows). |
| Pip | ✅ Поиск python/pip, pip install --target, метаданные и entry points, копирование в layout, генерация package.py; санитизация путей (Windows). |
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
| Резолвер | PubGrub + Rez backend | ~80% (нет filters/orderers/locks) |
| Env + rex | pre/commands/post + stamp/solve | ~95% |
| CLI команды | Все нативные, без passthrough | 100% по покрытию команд |
| Build | Локальный build, системы сборки | ~75% (нет central, часть REZ_BUILD_*) |
| Pip | Import в репо, entry points | ~80% |
| Context/Suite | .rxt, status, suite list/create | ~70% (нет полной семантики visibility) |
| Тесты пакетов | pre_test + tests, inplace | ~90% |
| Caching | pkg-cache (clear), нет resolve/memcache | ~30% |

**Итог:** для типичного сценария (package.py, resolve, env, pre/commands/post, test, build, pip, bind, context, suite) парность **~85%**. Не хватает в основном: filters/orderers/locks в резолвере, shell plugins, кэши резолва, центральный build и мелкие детали конфига/схемы.

---

## Ссылки

- [PLAN.md](PLAN.md) — приоритеты и порядок работ.
- [STATUS.md](STATUS.md) — статус каждой команды.
- [TODO.md](TODO.md) — детальный Rez parity roadmap.
