# Token Expansion

Variable references in environment values.

## Syntax

Use `{VARNAME}` to reference other variables:

```python
env.add(Evar("ROOT", "/opt/maya", "set"))
env.add(Evar("BIN", "{ROOT}/bin", "set"))
# BIN = /opt/maya/bin
```

## Resolution Order

Tokens are resolved in definition order:

```python
env.add(Evar("A", "1", "set"))
env.add(Evar("B", "{A}2", "set"))    # B = 12
env.add(Evar("C", "{B}3", "set"))    # C = 123
```

## System Variables

Reference existing environment variables:

```python
env.add(Evar("PATH", "{PATH};/new/path", "set"))
# Appends to existing PATH
```

## Recursive Expansion

Tokens can reference tokens (up to 10 levels):

```python
env.add(Evar("BASE", "/opt", "set"))
env.add(Evar("APP", "{BASE}/maya", "set"))
env.add(Evar("BIN", "{APP}/bin", "set"))
env.add(Evar("PATH", "{BIN}", "append"))
# PATH gets /opt/maya/bin
```

## API

```python
# Manual expansion
solved_env = env.solve()

# With max depth
solved_env = env.solve(max_depth=5)

# Check for unresolved
for evar in solved_env.evars:
    if "{" in evar.value:
        print(f"Unresolved: {evar.name}")
```

## CLI

```powershell
# Print with expansion (default)
pkg env maya

# Print without expansion
pkg env maya -e false

# With PKG_* stamp variables
pkg env maya -s
```
