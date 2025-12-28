# Installation

## Requirements

- Rust 1.70+ (for building)
- Python 3.8+ (for package definitions)

## Building from Source

### CLI Binary

```powershell
# Release build (recommended)
.\bootstrap.ps1 build

# Debug build
.\bootstrap.ps1 build -d
```

Binary location: `target/release/pkg.exe` (or `target/debug/pkg.exe`)

### Python Module

```powershell
# Build and install in current venv
.\bootstrap.ps1 python -i

# Build wheel only
.\bootstrap.ps1 python
```

Wheel location: `target/wheels/pkg-*.whl`

## Verification

```powershell
# CLI
pkg --version
pkg --help

# Python
python -c "from pkg import Package; print('OK')"
```

## Shell Completions

```powershell
# PowerShell (add to $PROFILE)
pkg completions powershell >> $PROFILE

# Bash
pkg completions bash >> ~/.bashrc

# Zsh
pkg completions zsh >> ~/.zshrc

# Fish
pkg completions fish > ~/.config/fish/completions/pkg.fish
```
