# Порядок внедрения: выравнивание с rez-next

Дата: 2026-02-08

## Зависимости

```
Package (схема, требования, validate)
    ↓
Repository (trait, Manager, Metadata, Stats)  →  Storage использует
    ↓
ResolvedContext (.rxt, env, методы)           →  solver result + Package
    ↓
Build (BuildManager, BuildRequest, Process)   →  использует Context + Package
```

## Порядок работ

### Этап 1: Package — схема и API ✅
1. ✅ Добавлены поля `format_version: Option<i32>`, `preprocess: Option<String>` (остальные уже были).
2. ✅ Введён тип `PackageRequirement` в dep.rs (name, version_spec, weak), `parse()`, `satisfied_by()`, реэкспорт из lib.
3. ✅ Добавлен `Package::validate()` (Rust) и `validate_py()` (Python): имя, формат имени, версия, непустые reqs/build_requires/private_build_requires/variants.
4. ✅ Обратная совместимость сохранена.

### Этап 2: Repository — интерфейс ✅
1. ✅ `RepositoryMetadata`, `PackageSearchCriteria`, `RepositoryStats` (package_count, version_count, variant_count, last_scan_*).
2. ✅ Trait `PackageRepository` расширен: `metadata()`, `is_initialized()`, `refresh()`, `find_packages()`, `get_package()`, `get_package_versions()`, `package_exists()`, `get_package_names()`, `get_stats()`.
3. ✅ `FilesystemRepository` и `MemoryRepository` реализуют trait (sync).
4. ✅ `RepositoryManager`: `add_repository()`, `find_packages()`, `get_package()`, `initialize_all()`, `refresh_all()`.
5. Storage по-прежнему использует `scan_repositories()`; новый API доступен для прямого использования.

### Этап 3: ResolvedContext и .rxt ✅
1. ✅ Модуль `context.rs`: `ResolvedContext` (id, requirements, resolved_packages, environment_vars, metadata, created_at, suite, platform, arch, status, failure_description).
2. ✅ Методы: `get_package()`, `get_package_names()`, `has_package()`, `get_environ()`, `get_tools()`.
3. ✅ `to_rxt_json()` / `to_rxt_string()` / `save_to_file()` / `load_from_file()`.
4. ✅ `pkg env --save-context <path.rxt>` сохраняет контекст после resolve.

### Этап 4: Build — интерфейс ✅
1. ✅ `BuildConfig` (build_dir, temp_dir, max_concurrent_builds, timeout, clean_before_build, verbosity, build_env_vars), `BuildVerbosity`.
2. ✅ `BuildRequest` (package, source_dir, variant, options, install_path), `BuildProcessResult` (request + report).
3. ✅ `BuildManager` с `build(request, storage)` — вызывает `build_package()`.
4. ✅ Текущий CLI `pkg build` и `build_package()` без изменений.

---

## Итог (все этапы выполнены)

- **Package:** поля format_version, preprocess; тип PackageRequirement + parse/satisfied_by; validate() / validate_py().
- **Repository:** RepositoryMetadata, PackageSearchCriteria, RepositoryStats; расширенный trait PackageRepository; RepositoryManager; тесты.
- **ResolvedContext:** тип в context.rs, save/load .rxt, get_package/get_environ/get_tools; `pkg env --save-context path.rxt`.
- **Build:** BuildConfig, BuildVerbosity, BuildRequest, BuildProcessResult, BuildManager::build().

Тесты: 120 passed. CLI `pkg build` и `pkg env` работают как раньше; новый API доступен через lib.
