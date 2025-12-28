# App

Application entry point definition.

## Constructor

```python
from pkg import App

app = App(name: str)
```

## Properties

| Property | Type | Description |
|----------|------|-------------|
| `name` | str | Application name |
| `path` | str | Executable path |
| `env_name` | str | Environment to use |
| `args` | list[str] | Default arguments |
| `cwd` | str | Working directory |
| `properties` | dict | Custom metadata |

## Builder Methods

```python
app = App("maya") \
    .with_path("/opt/maya/bin/maya") \
    .with_env("default") \
    .with_cwd("/projects") \
    .with_arg("-batch") \
    .with_property("icon", "maya.png")
```

## Example

```python
from pkg import App

# Basic app
app = App("houdini")
app.path = "/opt/hfs/bin/houdini"
app.env_name = "default"

# Batch renderer with arguments
render = App("hrender")
render.path = "/opt/hfs/bin/hrender"
render.args = ["-e", "-v"]
render.cwd = "/renders"

# Store metadata
app.properties["category"] = "DCC"
app.properties["icon"] = "houdini.svg"
app.properties["description"] = "Procedural 3D"
```

## Adding to Package

```python
from pkg import Package, App

pkg = Package("houdini", "21.0.0")

# Add multiple apps
pkg.add_app(App("houdini").with_path("/opt/hfs/bin/houdini"))
pkg.add_app(App("hython").with_path("/opt/hfs/bin/hython"))
pkg.add_app(App("hrender").with_path("/opt/hfs/bin/hrender"))

# Get app
app = pkg.get_app("hython")
```
