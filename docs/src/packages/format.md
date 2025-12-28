# package.py Format

Every package is defined by a `package.py` file containing a `get_package()` function.

## Minimal Example

```python
from packager import Package

def get_package():
    return Package("mytool", "1.0.0")
```

## Full Example

```python
from packager import Package, Env, Evar, App
from pathlib import Path
import sys

def get_package():
    pkg = Package("maya", "2024.0.0")
    
    # Platform-specific root
    if sys.platform == "win32":
        root = Path("C:/Program Files/Autodesk/Maya2024")
    else:
        root = Path("/usr/autodesk/maya2024")
    
    # Environment
    env = Env("default")
    env.add(Evar("MAYA_LOCATION", str(root), "set"))
    env.add(Evar("PATH", "{MAYA_LOCATION}/bin", "append"))
    env.add(Evar("PYTHONPATH", "{MAYA_LOCATION}/scripts", "append"))
    pkg.add_env(env)
    
    # Applications
    exe = ".exe" if sys.platform == "win32" else ""
    
    app = App("maya")
    app.path = str(root / "bin" / f"maya{exe}")
    app.env_name = "default"
    pkg.add_app(app)
    
    app2 = App("mayapy")
    app2.path = str(root / "bin" / f"mayapy{exe}")
    pkg.add_app(app2)
    
    # Dependencies
    pkg.add_req("arnold@>=5.0")
    pkg.add_req("redshift@>=3.5,<4.0")
    
    return pkg
```

## Available in Scope

The following are automatically available:

- `Package`, `Env`, `Evar`, `App` - Core classes
- `sys`, `os`, `pathlib.Path` - Standard modules
- Full Python standard library

## Function Signature

```python
def get_package(*args, **kwargs):
    ...
```

Arguments are reserved for future use (context passing).
