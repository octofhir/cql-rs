# CQL-RS Development Justfile
# Usage: just <command>
# Install just: cargo install just

# List all available commands
default:
    @just --list

# ============================================================================
# Building
# ============================================================================

# Build all crates
build:
    cargo build

# Build with release optimizations
build-release:
    cargo build --release

# Build CLI binary only
build-cli:
    cargo build -p octofhir-cql --bin cql

# Build CLI with release optimizations
build-cli-release:
    cargo build --release -p octofhir-cql --bin cql

# ============================================================================
# Testing
# ============================================================================

# Run all tests
test:
    cargo test --workspace

# Test CLI functionality
test-cli:
    cargo test -p octofhir-cql --bin cql

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run specific test
test-specific name:
    cargo test {{name}}

# ============================================================================
# CQFramework Spec Tests & Compliance
# ============================================================================

# Run CQFramework specification tests
spec-tests:
    cargo test -p octofhir-cql --test cqframework_spec_tests -- --nocapture

# Generate compliance report
compliance-report:
    @echo "Generating CQL Specification Compliance Report..."
    cargo test -p octofhir-cql --test cqframework_spec_tests generate_compliance_report -- --nocapture
    @echo ""
    @echo "Report saved to: crates/octofhir-cql/compliance_report.md"

# Run spec tests and show summary
spec-summary:
    @cargo test -p octofhir-cql --test cqframework_spec_tests generate_compliance_report -- --nocapture 2>&1 | grep -E "(Total|Passed|Failed|Skipped|^##|^###|^\|)"

# Run logical operator tests (should be 100%)
spec-logical:
    cargo test -p octofhir-cql --test cqframework_spec_tests test_logical_operators -- --nocapture

# Run all spec tests including ignored ones
spec-all:
    cargo test -p octofhir-cql --test cqframework_spec_tests -- --nocapture --include-ignored

# ============================================================================
# Code Quality
# ============================================================================

# Check compilation
check:
    cargo check --workspace

# Check with all features
check-all:
    cargo check --workspace --all-features

# Run clippy linter
lint:
    cargo clippy --workspace --all-targets

# Auto-fix clippy issues
lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty

# Format code
format:
    cargo fmt --all

# Check formatting without applying
format-check:
    cargo fmt --all -- --check

# Auto-fix all issues (clippy + format)
fix: lint-fix format

# ============================================================================
# CLI Commands
# ============================================================================

# Install CLI binary
install:
    cargo install --path crates/octofhir-cql

# Run CLI with arguments
run-cli *args:
    cargo run -p octofhir-cql --bin cql -- {{args}}

# Show CLI help
cli-help:
    cargo run -p octofhir-cql --bin cql -- --help

# Show CLI version
cli-version:
    cargo run -p octofhir-cql --bin cql -- --version

# Validate example CQL files
example-validate:
    cargo run -p octofhir-cql --bin cql -- validate examples/*.cql

# Translate example to ELM
example-translate:
    cargo run -p octofhir-cql --bin cql -- translate examples/simple.cql --pretty

# Start REPL
example-repl:
    cargo run -p octofhir-cql --bin cql -- repl

# ============================================================================
# Documentation
# ============================================================================

# Generate and open documentation
docs:
    cargo doc --workspace --no-deps --open

# Generate docs with private items
docs-all:
    cargo doc --workspace --document-private-items

# ============================================================================
# Cleaning
# ============================================================================

# Clean all build artifacts
clean:
    cargo clean

# Clean specific crate dependencies
clean-deps:
    cargo clean -p octofhir-cql-ast
    cargo clean -p octofhir-cql-parser
    cargo clean -p octofhir-cql-elm
    cargo clean -p octofhir-cql-types
    cargo clean -p octofhir-cql-eval
    cargo clean -p octofhir-cql-model
    cargo clean -p octofhir-cql

# ============================================================================
# Development Workflows
# ============================================================================

# CI workflow (check, test, lint, format-check)
ci: check test lint format-check

# Complete workflow (clean, build, test, check, lint, format, docs)
all: clean build test check lint format docs

# Quick development cycle
dev: check test-cli

# Release preparation
release: clean format lint test build-release
    @echo "Release build complete!"
    @ls -lh target/release/cql

# ============================================================================
# Watch & Monitor
# ============================================================================

# Watch for changes and run checks (requires cargo-watch)
watch:
    cargo watch -x check -x test

# Watch and run CLI (requires cargo-watch)
watch-cli *args:
    cargo watch -x "run -p octofhir-cql --bin cql -- {{args}}"

# ============================================================================
# Benchmarking & Profiling
# ============================================================================

# Run benchmarks (if benchmarks exist)
bench:
    cargo bench

# Generate code coverage (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --workspace --out Html --output-dir coverage

# Analyze binary size (requires cargo-bloat)
bloat:
    cargo bloat --release -p octofhir-cql --bin cql

# Profile build and show binary size
profile-build: build-cli-release
    strip target/release/cql
    @echo "Binary size:"
    @ls -lh target/release/cql

# ============================================================================
# Dependencies
# ============================================================================

# Update dependencies
update:
    cargo update

# Check for outdated dependencies (requires cargo-outdated)
update-check:
    cargo outdated

# Show dependency tree
deps-tree:
    cargo tree -p octofhir-cql

# ============================================================================
# Git & Publishing
# ============================================================================

# Show git status
status:
    git status

# Create a new commit with all changes
commit message:
    git add .
    git commit -m "{{message}}"

# Publish dry run
publish-dry:
    cargo publish -p octofhir-cql --dry-run

# Publish to crates.io
publish:
    cargo publish -p octofhir-cql

# ============================================================================
# Shortcuts
# ============================================================================

# Quick: check and test
q: check test-cli

# Build and run CLI
br *args: build-cli
    ./target/debug/cql {{args}}

# Build release and run CLI
brr *args: build-cli-release
    ./target/release/cql {{args}}

# Validate a specific CQL file
validate file: build-cli
    ./target/debug/cql validate {{file}}

# Translate a specific CQL file
translate file: build-cli
    ./target/debug/cql translate {{file}} --pretty

# Execute a specific CQL file
execute file *args: build-cli
    ./target/debug/cql execute {{file}} {{args}}

# Start REPL
repl: build-cli
    ./target/debug/cql repl
