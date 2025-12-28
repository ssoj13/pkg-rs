# Env & Evar

Environment and variable classes.

## Evar

Single environment variable.

```python
from packager import Evar

# Constructor
evar = Evar(name: str, value: str, action: str = "set")

# Actions: "set", "append", "insert"
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | str | Variable name |
| `value` | str | Variable value |
| `action` | str | set/append/insert |

## Env

Collection of environment variables.

```python
from packager import Env, Evar

env = Env("default")
env.add(Evar("ROOT", "/opt/tool", "set"))
env.add(Evar("PATH", "{ROOT}/bin", "append"))
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | str | Environment name |
| `evars` | list[Evar] | Variables |

## Methods

```python
# Add variable
env.add(evar)

# Get by name
evar = env.get("PATH")

# Expand tokens
solved = env.solve()

# Apply to os.environ
env.commit()

# Serialize
json_str = env.to_json()
```

## Token Expansion

```python
env = Env("default")
env.add(Evar("ROOT", "/opt/maya", "set"))
env.add(Evar("BIN", "{ROOT}/bin", "set"))
env.add(Evar("LIB", "{ROOT}/lib", "set"))

solved = env.solve()
# BIN = /opt/maya/bin
# LIB = /opt/maya/lib
```

## Complete Example

```python
from packager import Env, Evar

# Create environment
env = Env("production")

# Set variables
env.add(Evar("APP_ROOT", "/opt/myapp", "set"))
env.add(Evar("APP_CONFIG", "{APP_ROOT}/config", "set"))
env.add(Evar("PATH", "{APP_ROOT}/bin", "append"))
env.add(Evar("LD_LIBRARY_PATH", "{APP_ROOT}/lib", "append"))

# Expand and apply
solved = env.solve()
solved.commit()

# Now os.environ contains the variables
import os
print(os.environ["APP_ROOT"])
```
