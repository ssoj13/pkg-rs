# Shell Completions

Generate and install shell completions for tab-completion support.

## PowerShell

```powershell
# Add to profile
pkg completions powershell >> $PROFILE

# Or source directly
pkg completions powershell | Out-String | Invoke-Expression
```

## Bash

```bash
# Add to .bashrc
pkg completions bash >> ~/.bashrc

# Or source directly
source <(pkg completions bash)
```

## Zsh

```zsh
# Add to .zshrc
pkg completions zsh >> ~/.zshrc

# Or use completions directory
pkg completions zsh > ~/.zsh/completions/_pkg
```

## Fish

```fish
pkg completions fish > ~/.config/fish/completions/pkg.fish
```

## Elvish

```elvish
pkg completions elvish >> ~/.elvish/rc.elv
```

## What Gets Completed

- Command names (`list`, `info`, `solve`, etc.)
- Package names (after `info`, `solve`, `env`, `run`)
- Option flags (`--json`, `--dry-run`, etc.)
