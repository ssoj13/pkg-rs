# Applications

Applications define executable entry points.

## Basic App

```python
from packager import App

app = App("maya")
app.path = "/opt/maya/bin/maya"
pkg.add_app(app)
```

## Full Configuration

```python
app = App("maya")
app.path = "/opt/maya/bin/maya"
app.env_name = "default"        # Environment to use
app.cwd = "/projects"           # Working directory
app.args = ["-batch"]           # Default arguments
pkg.add_app(app)
```

## Multiple Apps

```python
# Main application
pkg.add_app(App("maya").with_path("/opt/maya/bin/maya"))

# Python interpreter
pkg.add_app(App("mayapy").with_path("/opt/maya/bin/mayapy"))

# Batch renderer
render = App("render")
render.path = "/opt/maya/bin/render"
render.args = ["-r", "arnold"]
pkg.add_app(render)
```

## Builder Pattern

```python
app = App("houdini") \
    .with_path("/opt/hfs/bin/houdini") \
    .with_env("default") \
    .with_cwd("/projects") \
    .with_arg("-foreground")
```

## Running Apps

```powershell
# Default app (first defined)
pkg run maya

# Specific app
pkg run maya mayapy

# With extra arguments
pkg run maya -- -batch -file scene.ma

# Dry run (show what would run)
pkg run maya -n
```

## Properties

Store metadata for tools/GUIs:

```python
app.properties["icon"] = "maya.png"
app.properties["category"] = "DCC"
app.properties["description"] = "3D Animation"
```
