# ASCII Diagrams

## Build Pipeline (ASCII)

```
package.py + source tree + CLI flags + repos
  |
  v
Load Package -> Collect Variants -> Select Variant(s)
  |
  v
Resolve Build Context -> Set REZ_BUILD_* env vars
  |
  v
Execute pre_build_commands (env mutations)
  |
  v
Run Build System (custom/make/cmake) or emit build scripts
  |
  +--> build.rxt snapshot + variant.json
  |
  v
Install payload + package.py + variant metadata (optional)
```

## Pip Import Pipeline (ASCII)

```
package spec + CLI flags + repos
  |
  v
Find Python/Pip -> pip install --target (temp)
  |
  v
Parse dist-info metadata -> Build rez requirements + variants
  |
  v
Copy payload into repo layout (hashed variant subpath)
  |
  v
Generate entry points -> Write package.py
```

## Variant Layout (ASCII)

```
Package Root
  |
  v
hashed_variants?
  |-- yes --> SHA1 of python list repr -> hashed subpath
  |-- no  --> Join variant requirements -> readable subpath
```
