# Justfile Quick Start Guide

This project uses [`just`](https://github.com/casey/just) as a modern command runner, similar to `make` but simpler and more user-friendly.

## Installation

```bash
# Install just using cargo
cargo install just

# Or on macOS using Homebrew
brew install just

# Or on Linux using package manager
# Ubuntu/Debian
sudo apt install just

# Arch Linux
sudo pacman -S just
```

## Usage

### List all available commands

```bash
just
# or
just --list
```

This shows all 50+ commands available!

### Most Common Commands

```bash
# Building
just build                    # Build all crates
just build-cli                # Build CLI only
just build-release            # Build with optimizations

# Testing
just test                     # Run all tests
just test-cli                 # Test CLI
just q                        # Quick: check + test (fast!)

# Code Quality
just check                    # Check compilation
just lint                     # Run clippy
just format                   # Format code
just fix                      # Auto-fix issues

# CLI Operations
just cli-help                 # Show CLI help
just validate FILE            # Validate a CQL file
just translate FILE           # Translate CQL to ELM
just execute FILE [ARGS]      # Execute a CQL file
just repl                     # Start REPL

# Development Shortcuts
just br ARGS                  # Build and Run CLI (debug)
just brr ARGS                 # Build and Run CLI (release)

# Complete Workflows
just ci                       # Run CI checks (check, test, lint, format-check)
just release                  # Prepare release build
just dev                      # Quick dev cycle
```

## Examples

### Quick Development Cycle

```bash
# 1. Check and test
just q

# 2. Build and validate a file
just validate examples/measure.cql

# 3. Translate to ELM
just translate examples/measure.cql

# 4. Start REPL for testing
just repl
```

### Before Committing

```bash
# Run all checks
just ci

# Or fix issues automatically
just fix

# Then commit
just commit "your message"
```

### Working with CLI

```bash
# Build and run in one command
just br --help
just br validate test.cql
just br execute test.cql --param Age=65

# Or with release build (faster)
just brr execute large-file.cql
```

### Full Development Workflow

```bash
# Clean build everything
just clean
just all

# Or just the essentials
just dev
```

## Advanced Features

### Running with Arguments

```bash
# Execute with custom arguments
just execute measure.cql --param Age=65 --data patient.json

# Run CLI with any arguments
just run-cli translate measure.cql --pretty
```

### Watching for Changes

```bash
# Auto-run checks on file changes (requires cargo-watch)
just watch

# Watch and run CLI
just watch-cli validate test.cql
```

### Documentation

```bash
# Generate and open docs
just docs

# Show dependency tree
just deps-tree
```

### Release Preparation

```bash
# Complete release workflow
just release

# Shows binary size and runs all checks
```

## Why `just` instead of `make`?

1. **Simpler syntax** - No tabs vs spaces issues
2. **Better error messages** - More helpful when things go wrong
3. **Cross-platform** - Works the same on Windows, Mac, Linux
4. **Modern features** - String interpolation, better parameter handling
5. **Just for commands** - Not a build system, just a command runner

## Tips

### Tab Completion

Enable tab completion for better UX:

```bash
# For bash
just --completions bash > ~/.local/share/bash-completion/completions/just

# For zsh
just --completions zsh > ~/.zsh/completions/_just

# For fish
just --completions fish > ~/.config/fish/completions/just.fish
```

### Aliases

Create shell aliases for common commands:

```bash
# Add to .bashrc or .zshrc
alias jj='just'
alias jb='just build'
alias jt='just test'
alias jq='just q'
alias jr='just run-cli'
```

### Working Directory

`just` commands run from the project root, so you can run them from any subdirectory:

```bash
cd crates/octofhir-cql
just test          # Still works!
```

## Full Command Reference

See all commands with descriptions:

```bash
just --list
```

Or check the [justfile](justfile) directly to see the implementation.

## Learn More

- [just Documentation](https://just.systems/)
- [just GitHub](https://github.com/casey/just)
- [Comparison with make](https://github.com/casey/just#what-are-the-idiosyncrasies-of-make-that-just-avoids)

---

**Happy coding with `just`!** 🚀
