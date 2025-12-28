# Package

Core class representing a software package.

## Constructor

```python
pkg = Package(base: str, version: str)
```

## Properties

| Property | Type | Description |
|----------|------|-------------|
| `name` | str | Full name (`base-version`) |
| `base` | str | Package identifier |
| `version` | str | SemVer version |
| `reqs` | list[str] | Requirements (constraints) |
| `deps` | list[Package] | Resolved dependencies |
| `envs` | list[Env] | Environments |
| `apps` | list[App] | Applications |
| `tags` | list[str] | Tags for filtering |

## Methods

```python
# Add requirement
pkg.add_req("redshift@>=3.5")

# Add environment
pkg.add_env(env)

# Add application
pkg.add_app(app)

# Add tag
pkg.add_tag("dcc")

# Get environment by name
env = pkg.get_env("default")

# Get app by name
app = pkg.get_app("maya")

# Solve dependencies
pkg.solve(available_packages)

# Get effective environment (merged with deps)
env = pkg.effective_env("default")
```

## Example

```python
from pkg import Package, Env, Evar, App

pkg = Package("mytool", "1.0.0")

# Requirements
pkg.add_req("python@>=3.9")
pkg.add_req("numpy")

# Environment
env = Env("default")
env.add(Evar("TOOL_ROOT", "/opt/mytool", "set"))
pkg.add_env(env)

# App
app = App("mytool")
app.path = "/opt/mytool/bin/run"
pkg.add_app(app)

# Tags
pkg.add_tag("tools")
pkg.add_tag("python")
```
