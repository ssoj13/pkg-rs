# Mermaid Diagrams

## Config Precedence

```mermaid
flowchart TD
    A[Defaults] --> B[Config files list]
    B --> C[Home config]
    C --> D[Env overrides: PKG_*]
    D --> E[Env overrides: PKG_*_JSON]
    E --> F[Package config section (build/release)]
    F --> G[Effective Config]
```

## CLI Command Routing

```mermaid
flowchart LR
    CLI[pkg CLI] --> CMD[Command Dispatcher]
    CMD --> ENV[pkg env]
    CMD --> BUILD[pkg build]
    CMD --> PIP[pkg pip]
    CMD --> LIST[pkg list/info/scan]
    CMD --> CTX[pkg context/suite/status]

    ENV --> RESOLVE[Resolver]
    RESOLVE --> CTXOBJ[ResolvedContext]

    BUILD --> BUILDPIPE[Build Pipeline]
    PIP --> PIPPIPE[Pip Import]
    LIST --> STORAGE[Storage Scan]
```

## Resolve + Context Pipeline

```mermaid
flowchart TD
    REQ[Package Requests] --> FILTERS[Filters + Orderers + Timestamp]
    FILTERS --> SOLVER[Resolver/Solver]
    SOLVER --> CTX[ResolvedContext]
    CTX --> SHELL[Shell Env Output]
    CTX --> RXT[.rxt Serialization]
    CTX --> EXEC[Command Execution]
```

## Build Pipeline

```mermaid
flowchart TD
    PKG[Developer Package] --> DETECT[BuildSystem Detection]
    DETECT --> PROC[BuildProcess (local/central)]
    PROC --> BCTX[Resolve Build Context]
    BCTX --> ENVVARS[Set REZ_BUILD_* + pre_build_commands]
    ENVVARS --> PHASES[Configure/Build/Install]
    PHASES --> INSTALL[Install Payload + Metadata]
    PHASES --> SCRIPTS[build-env + build.rxt]
```

## Pip Pipeline

```mermaid
flowchart TD
    SPEC[Pip Spec] --> FINDPY[Find rezified python/pip]
    FINDPY --> PIPINSTALL[pip install --target]
    PIPINSTALL --> META[dist-info + RECORD + entry points]
    META --> REQS[PEP440 -> Rez requirements]
    REQS --> COPY[Copy payload into repo layout]
    COPY --> PKGDEF[Write package.py + metadata]
```