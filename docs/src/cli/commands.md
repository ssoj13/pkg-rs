# Commands

## list

List available packages.

```powershell
pkg list              # All packages
pkg list -L           # Latest versions only
pkg list -n maya      # Filter by name
pkg list -t dcc       # Filter by tag
pkg list --json       # JSON output
```

## info

Show package details.

```powershell
pkg info maya             # Latest version
pkg info maya-2024.0.0    # Specific version
pkg info maya --json      # JSON output
```

## solve

Resolve dependencies using SAT solver.

```powershell
pkg solve maya                # Single package
pkg solve maya houdini        # Multiple packages
pkg solve maya -n             # Dry run (preview)
pkg solve maya --json         # JSON output
```

## env

Print environment variables for package(s).

```powershell
pkg env maya              # Print env (tokens expanded by default)
pkg env maya -e false     # Without token expansion
pkg env maya -s           # Include PKG_* stamp variables
pkg env maya -f json      # JSON format
pkg env maya -o env.ps1   # Export to file
pkg env maya bifrost arnold  # Multiple packages (toolset)
```

**Options:**
- `-e, --expand` - Expand `{TOKEN}` references (default: true)
- `-s, --stamp` - Add PKG_* variables for each package (default: false)
- `-f, --format` - Output format: shell, json, export, set
- `-o, --output` - Write to file
- `-n, --dry-run` - Preview what would be set

**PATH Order:** Direct requirements appear first (in request order), then transitive dependencies.

## run

Launch application from package.

```powershell
pkg run maya                      # Default app
pkg run maya mayapy               # Specific app
pkg run maya -n                   # Dry run
pkg run maya -- -batch -file x.ma # With arguments
```

## graph

Visualize dependency graph.

```powershell
pkg graph maya                # DOT format
pkg graph maya -f mermaid     # Mermaid format
pkg graph maya -R             # Reverse deps
pkg graph maya -d 2           # Limit depth
```

## scan

Scan locations and show statistics.

```powershell
pkg scan              # Default locations
pkg scan ./repo       # Specific path
```

## shell

Interactive mode with tab completion.

```powershell
pkg shell
pkg sh          # Alias
```

## py

Python REPL with pkg module.

```powershell
pkg py                    # Interactive REPL
pkg py script.py          # Run script
pkg py script.py -- -v    # With arguments
```

## gen-repo

Generate test repository with random packages for stress-testing.

```powershell
pkg gen-repo                     # Medium preset (50 x 3)
pkg gen-repo --small             # 10 packages x 2 versions
pkg gen-repo --large             # 200 packages x 5 versions
pkg gen-repo --stress            # 1000 packages x 10 versions
pkg gen-repo -n 100 -V 5         # Custom: 100 packages, 5 versions
pkg gen-repo -o ./my-repo        # Custom output directory
pkg gen-repo --seed 42           # Reproducible generation
pkg gen-repo --dep-rate 0.5      # 50% dependency probability
```

Always includes core packages: maya, houdini, nuke, aftereffects, resolve, arnold, vray, redshift, usd, python.

## completions

Generate shell completions.

```powershell
pkg completions powershell
pkg completions bash
pkg completions zsh
pkg completions fish
```
