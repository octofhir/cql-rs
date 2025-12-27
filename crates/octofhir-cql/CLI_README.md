# CQL Command-Line Interface

A production-ready CLI for working with Clinical Quality Language (CQL) files.

## Features

- ✅ **Execute** - Run CQL files with parameters and context data
- ✅ **Translate** - Convert CQL to ELM (JSON/XML)
- ✅ **Validate** - Check syntax and semantics
- ✅ **REPL** - Interactive mode for testing expressions
- ✅ **Library Resolution** - Automatic dependency resolution with caching
- ✅ **Multiple Output Formats** - JSON, pretty JSON, table
- ✅ **Colored Output** - Better readability in terminal

## Installation

### From Source

```bash
cargo install --path .
```

### Build Only

```bash
cargo build --release --bin cql
# Binary at: target/release/cql
```

## Quick Start

```bash
# Validate a CQL file
cql validate MyMeasure.cql

# Translate to ELM
cql translate MyMeasure.cql --pretty

# Execute with data
cql execute MyMeasure.cql --data patient.json

# Start interactive REPL
cql repl
```

## Commands

### Execute

Run a CQL file with parameters and context data.

```bash
cql execute <file.cql> [OPTIONS]

Options:
  -p, --param <NAME=VALUE>     Parameters (can be repeated)
  -d, --data <FILE>            Context data (JSON file)
  -L, --library-path <PATH>    Library search paths
  -v, --verbose                Verbose output
  -f, --format <FORMAT>        Output format (json, pretty, table)
  -o, --output <FILE>          Output file (default: stdout)
```

**Examples:**

```bash
# Simple execution
cql execute measure.cql

# With parameters
cql execute measure.cql \
  --param MeasurementPeriod=@2024 \
  --param MinAge=18

# With context data
cql execute measure.cql --data patient.json

# Save results
cql execute measure.cql --output results.json
```

### Translate

Convert CQL to ELM format.

```bash
cql translate <file.cql> [OPTIONS]

Options:
  -F, --format <FORMAT>        Output format (json, xml) [default: json]
  -p, --pretty                 Pretty-print output
  -a, --annotations            Include annotations
  -L, --library-path <PATH>    Library search paths
  -o, --output <FILE>          Output file (default: stdout)
```

**Examples:**

```bash
# Translate to JSON
cql translate measure.cql --pretty

# Translate to XML
cql translate measure.cql --format xml --output measure.xml

# With annotations
cql translate measure.cql --annotations --pretty
```

### Validate

Check CQL syntax and semantics.

```bash
cql validate <file.cql>... [OPTIONS]

Options:
  -s, --strict                 Strict mode (warnings as errors)
  -L, --library-path <PATH>    Library search paths
  -v, --verbose                Verbose output
```

**Examples:**

```bash
# Validate single file
cql validate measure.cql

# Validate multiple files
cql validate *.cql

# Strict mode
cql validate measure.cql --strict
```

### REPL

Interactive mode for testing CQL expressions.

```bash
cql repl [OPTIONS]

Options:
  -m, --model <MODEL>          Data model [default: FHIR]
  -V, --version <VERSION>      Model version
  -L, --library-path <PATH>    Library search paths
```

**REPL Commands:**

```
:help                Show help
:quit, :q            Quit REPL
:load <file>         Load a library file
:list, :ls           List all definitions
:clear, :c           Clear all definitions
:type <expr>         Show type of expression
:paths               Show library search paths

define X: expr       Define a named expression
expr                 Evaluate expression
```

**Example Session:**

```
$ cql repl
CQL Interactive REPL
Type :help for help, :quit to quit

cql> define Age: 65
Success: Definition added

cql> Age >= 18
true

cql> :load CommonLibrary.cql
Success: Loaded library: CommonLibrary version 1.0.0 (5 definitions)

cql> :list
Definitions:
  Age = 65
  InPopulation = (from CommonLibrary.cql)
  ...

cql> :quit
Goodbye!
```

## Global Options

Available for all commands:

```
-v, --verbose          Verbose output
-f, --format <FORMAT>  Output format (json, pretty, table)
-o, --output <FILE>    Output file (default: stdout)
    --color <COLOR>    Color output (auto, always, never) [default: auto]
-h, --help             Print help
-V, --version          Print version
```

## Library Resolution

The CLI automatically resolves library dependencies using:

1. Explicit paths via `-L, --library-path`
2. `CQL_LIBRARY_PATH` environment variable
3. Current working directory

**Examples:**

```bash
# Using command-line option
cql execute measure.cql -L ./libraries -L ./vendor

# Using environment variable
export CQL_LIBRARY_PATH=./libraries:./vendor
cql execute measure.cql

# Multiple patterns supported
# Will search for: Common-1.0.0.cql, Common_1.0.0.cql, Common.1.0.0.cql, Common.cql
```

## Output Formats

### JSON (default)

```bash
cql execute measure.cql --format json
```

### Pretty JSON

```bash
cql execute measure.cql --format pretty
```

### Table

```bash
cql execute measure.cql --format table
```

## Parameter Syntax

Parameters support various types:

```bash
# Integers
--param Age=65

# Decimals
--param Score=98.5

# Booleans
--param IsActive=true

# Strings
--param Name=John

# Dates (@ prefix)
--param StartDate=@2024-01-01

# JSON objects
--param 'Config={"key": "value"}'

# JSON arrays
--param 'Items=[1, 2, 3]'
```

## Exit Codes

- `0` - Success
- `1` - Error (validation failed, execution error, etc.)

## Environment Variables

- `CQL_LIBRARY_PATH` - Colon-separated library search paths
- `RUST_LOG` - Log level (for debugging, development only)

## Examples

### Complete Workflow

```bash
# 1. Validate
cql validate MyMeasure.cql

# 2. Translate
cql translate MyMeasure.cql --output MyMeasure.elm.json --pretty

# 3. Execute
cql execute MyMeasure.cql \
  --data test-patient.json \
  --param MeasurementPeriod=@2024 \
  --output results.json
```

### Working with Libraries

```bash
# Project structure
my-project/
├── libraries/
│   ├── Common.cql
│   └── FHIRHelpers.cql
└── measures/
    └── MyMeasure.cql

# Set library path
export CQL_LIBRARY_PATH=./libraries

# Execute measure (automatically resolves dependencies)
cd measures
cql execute MyMeasure.cql --data ../test-data/patient.json
```

### Batch Processing

```bash
# Validate all files
for file in *.cql; do
  cql validate "$file" || echo "Failed: $file"
done

# Translate all files
for file in *.cql; do
  cql translate "$file" --pretty > "${file%.cql}.elm.json"
done
```

## Debugging

```bash
# Verbose output
cql execute measure.cql --verbose

# Check library paths
cql repl
> :paths

# Development debugging (with backtrace)
RUST_BACKTRACE=1 cql execute problematic.cql
```

## Performance

The CLI is optimized for performance:

- Library caching reduces redundant file reads
- Circular dependency detection prevents infinite loops
- Efficient parsing with zero-copy where possible

For best performance, use the release build:

```bash
cargo build --release --bin cql
./target/release/cql execute large-measure.cql
```

## Troubleshooting

### "Library not found"

Check library search paths:
```bash
cql repl
> :paths
```

Add explicit path:
```bash
cql execute measure.cql -L ./path/to/libraries
```

### Parse Errors

Use validate with verbose output:
```bash
cql validate measure.cql --verbose
```

### Execution Issues

Check parameters and data:
```bash
cql execute measure.cql --verbose
```

## Development

See the main project [CLI_COMMANDS.md](../../CLI_COMMANDS.md) for development commands and workflows.

## License

MIT OR Apache-2.0

## Links

- [Main Repository](https://github.com/octofhir/cql-rs)
- [CQL Specification](https://cql.hl7.org/)
- [Full Command Reference](../../CLI_COMMANDS.md)
