# Environments

Environments define variables that configure the runtime.

## Creating Environments

```python
from packager import Env, Evar

env = Env("default")
env.add(Evar("ROOT", "/opt/tool", "set"))
env.add(Evar("PATH", "{ROOT}/bin", "append"))
pkg.add_env(env)
```

## Multiple Environments

```python
# Default environment
default = Env("default")
default.add(Evar("MODE", "production", "set"))
pkg.add_env(default)

# Debug environment
debug = Env("debug")
debug.add(Evar("MODE", "debug", "set"))
debug.add(Evar("LOG_LEVEL", "verbose", "set"))
pkg.add_env(debug)
```

## Evar Actions

| Action | Behavior |
|--------|----------|
| `set` | Overwrite variable |
| `append` | Add to end with separator |
| `insert` | Add to start with separator |

```python
# set: VAR = value
Evar("ROOT", "/opt/tool", "set")

# append: VAR = $VAR;value (Win) or $VAR:value (Linux)
Evar("PATH", "/opt/tool/bin", "append")

# insert: VAR = value;$VAR (Win) or value:$VAR (Linux)
Evar("PATH", "/opt/tool/bin", "insert")
```

## Token Expansion

Reference other variables with `{VARNAME}`:

```python
env.add(Evar("ROOT", "/opt/maya", "set"))
env.add(Evar("BIN", "{ROOT}/bin", "set"))       # -> /opt/maya/bin
env.add(Evar("PATH", "{BIN}", "append"))
```

## Applying Environments

```python
# In Python
env.solve()    # Expand tokens
env.commit()   # Apply to os.environ

# From CLI
pkg env maya           # Print expanded (default)
pkg env maya -s        # With PKG_* stamp variables
pkg run maya           # Apply and launch
```
