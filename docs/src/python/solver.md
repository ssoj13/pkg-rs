# Solver

Dependency resolver using PubGrub SAT algorithm.

## Constructor

```python
from packager import Solver

solver = Solver(packages: list[Package])
```

## Methods

```python
# Solve single package
solution = solver.solve("maya-2024.0.0")
# Returns: ["maya-2024.0.0", "redshift-3.5.0", ...]

# Solve multiple requirements
solution = solver.solve_reqs(["maya@>=2024", "houdini"])
```

## Example

```python
from packager import Storage, Solver

storage = Storage.scan()
solver = Solver(storage.packages)

try:
    solution = solver.solve("maya-2024.0.0")
    print("Resolved packages:")
    for pkg_name in solution:
        print(f"  {pkg_name}")
except RuntimeError as e:
    print(f"Resolution failed: {e}")
```

## Conflict Handling

When versions conflict, the solver provides details:

```python
try:
    solver.solve_reqs(["maya@2024", "legacy-tool"])
except RuntimeError as e:
    print(e)
    # Version conflict for ocio:
    #   maya-2024.0.0 requires ocio@>=2.0
    #   legacy-tool-1.0.0 requires ocio@<2.0
```

## Using with Package

```python
pkg = Package("myproject", "1.0.0")
pkg.add_req("maya@>=2024")
pkg.add_req("redshift@>=3.5")

# Solve populates pkg.deps
pkg.solve(storage.packages)

for dep in pkg.deps:
    print(f"Dependency: {dep.name}")
```
