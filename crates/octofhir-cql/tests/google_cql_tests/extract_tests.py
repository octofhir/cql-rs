#!/usr/bin/env python3
"""
Extract test cases from Google CQL Go test files into JSON format.

This script parses Go test files from google/cql and extracts:
- Test function name
- Test case name
- CQL expression
- Expected result (as string representation)
"""

import re
import json
import sys
from pathlib import Path
from typing import List, Dict, Any, Optional

def parse_go_value(value_str: str) -> Any:
    """Parse a Go value string into a Python value representation."""
    value_str = value_str.strip()

    # Handle newOrFatal wrapper
    match = re.match(r'newOrFatal\(t,\s*(.+)\)', value_str, re.DOTALL)
    if match:
        return parse_go_value(match.group(1).strip())

    # Handle nil
    if value_str == 'nil':
        return None

    # Handle integers
    if re.match(r'^-?\d+$', value_str):
        return int(value_str)

    # Handle longs (int64)
    match = re.match(r'^int64\((-?\d+)\)$', value_str)
    if match:
        return {"type": "Long", "value": int(match.group(1))}

    # Handle floats
    if re.match(r'^-?\d+\.\d+$', value_str):
        return float(value_str)

    # Handle float64 cast
    match = re.match(r'^float64\((.+)\)$', value_str)
    if match:
        inner = match.group(1).strip()
        try:
            return float(inner)
        except:
            return {"type": "Decimal", "value": inner}

    # Handle booleans
    if value_str == 'true':
        return True
    if value_str == 'false':
        return False

    # Handle strings
    if value_str.startswith('"') and value_str.endswith('"'):
        return value_str[1:-1]

    # Handle Quantity
    match = re.match(r'result\.Quantity\{Value:\s*(.+),\s*Unit:\s*model\.(\w+)\}', value_str)
    if match:
        return {
            "type": "Quantity",
            "value": float(match.group(1)),
            "unit": match.group(2)
        }

    # Handle DateTime
    match = re.match(r'result\.DateTime\{Date:\s*time\.Date\((.+)\),\s*Precision:\s*model\.(\w+)\}', value_str)
    if match:
        return {
            "type": "DateTime",
            "args": match.group(1),
            "precision": match.group(2)
        }

    # Handle Date
    match = re.match(r'result\.Date\{Date:\s*time\.Date\((.+)\),\s*Precision:\s*model\.(\w+)\}', value_str)
    if match:
        return {
            "type": "Date",
            "args": match.group(1),
            "precision": match.group(2)
        }

    # Handle Time
    match = re.match(r'result\.Time\{Date:\s*time\.Date\((.+)\),\s*Precision:\s*model\.(\w+)\}', value_str)
    if match:
        return {
            "type": "Time",
            "args": match.group(1),
            "precision": match.group(2)
        }

    # Handle Interval
    match = re.match(r'result\.Interval\{(.+)\}', value_str, re.DOTALL)
    if match:
        return {
            "type": "Interval",
            "raw": match.group(1).strip()
        }

    # Handle List
    match = re.match(r'result\.List\{(.+)\}', value_str, re.DOTALL)
    if match:
        return {
            "type": "List",
            "raw": match.group(1).strip()
        }

    # Handle Tuple
    match = re.match(r'result\.Tuple\{(.+)\}', value_str, re.DOTALL)
    if match:
        return {
            "type": "Tuple",
            "raw": match.group(1).strip()
        }

    # Handle math constants
    if value_str == 'math.MaxInt32':
        return 2147483647
    if value_str == 'math.MinInt32':
        return -2147483648

    # Return raw for complex values
    return {"raw": value_str}

def extract_test_case(content: str, start: int) -> Optional[Dict[str, Any]]:
    """Extract a single test case starting at the given position."""
    # Find opening brace
    brace_pos = content.find('{', start)
    if brace_pos == -1:
        return None

    # Track brace depth to find matching closing brace
    depth = 1
    pos = brace_pos + 1
    while depth > 0 and pos < len(content):
        if content[pos] == '{':
            depth += 1
        elif content[pos] == '}':
            depth -= 1
        pos += 1

    if depth != 0:
        return None

    case_content = content[brace_pos:pos]

    # Extract name
    name_match = re.search(r'name:\s*"([^"]*)"', case_content)
    name = name_match.group(1) if name_match else None

    # Extract cql - handle backticks and regular strings
    cql_match = re.search(r'cql:\s*`([^`]*)`', case_content)
    if not cql_match:
        cql_match = re.search(r'cql:\s*"((?:[^"\\]|\\.)*)"', case_content)
    cql = cql_match.group(1) if cql_match else None

    # Extract wantResult
    want_result = None
    result_match = re.search(r'wantResult:\s*', case_content)
    if result_match:
        # Find the value after wantResult:
        val_start = result_match.end()
        # Find the end - either comma followed by newline, or closing brace
        val_end = val_start
        paren_depth = 0
        brace_depth = 0
        while val_end < len(case_content):
            c = case_content[val_end]
            if c == '(':
                paren_depth += 1
            elif c == ')':
                paren_depth -= 1
            elif c == '{':
                brace_depth += 1
            elif c == '}':
                if brace_depth == 0:
                    break
                brace_depth -= 1
            elif c == ',' and paren_depth == 0 and brace_depth == 0:
                # Check if this is end of value
                remaining = case_content[val_end+1:].lstrip()
                if remaining.startswith(('name:', 'cql:', 'wantModel:', 'wantResult:', '}')):
                    break
            val_end += 1

        want_result_str = case_content[val_start:val_end].strip().rstrip(',')
        want_result = parse_go_value(want_result_str)

    if name and cql:
        return {
            "name": name,
            "cql": cql,
            "expected": want_result
        }
    return None

def extract_tests_from_function(content: str, func_name: str) -> List[Dict[str, Any]]:
    """Extract all test cases from a test function."""
    tests = []

    # Find the function
    func_match = re.search(rf'func\s+{func_name}\s*\(t\s+\*testing\.T\)\s*\{{', content)
    if not func_match:
        return tests

    func_start = func_match.end()

    # Find tests := []struct
    struct_match = re.search(r'tests\s*:=\s*\[\]struct\s*\{[^}]+\}\s*\{', content[func_start:])
    if not struct_match:
        return tests

    array_start = func_start + struct_match.end()

    # Find closing of the array
    depth = 1
    pos = array_start
    while depth > 0 and pos < len(content):
        if content[pos] == '{':
            depth += 1
        elif content[pos] == '}':
            depth -= 1
        pos += 1

    array_content = content[array_start:pos-1]

    # Extract each test case
    case_start = 0
    while True:
        # Find next test case
        next_case = array_content.find('{', case_start)
        if next_case == -1:
            break

        case = extract_test_case(array_content, next_case)
        if case:
            tests.append(case)

        # Move past this case
        depth = 1
        pos = next_case + 1
        while depth > 0 and pos < len(array_content):
            if array_content[pos] == '{':
                depth += 1
            elif array_content[pos] == '}':
                depth -= 1
            pos += 1
        case_start = pos

    return tests

def extract_tests_from_file(filepath: Path) -> Dict[str, List[Dict[str, Any]]]:
    """Extract all tests from a Go test file."""
    content = filepath.read_text()

    # Find all test functions
    func_pattern = re.compile(r'func\s+(Test\w+)\s*\(t\s+\*testing\.T\)')
    functions = func_pattern.findall(content)

    all_tests = {}
    for func_name in functions:
        tests = extract_tests_from_function(content, func_name)
        if tests:
            all_tests[func_name] = tests

    return all_tests

def main():
    if len(sys.argv) < 2:
        print("Usage: extract_tests.py <go_test_file> [output_json]")
        sys.exit(1)

    input_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_file.exists():
        print(f"Error: {input_file} does not exist")
        sys.exit(1)

    tests = extract_tests_from_file(input_file)

    # Count total tests
    total = sum(len(t) for t in tests.values())
    print(f"Extracted {total} tests from {len(tests)} functions", file=sys.stderr)

    result = {
        "source": str(input_file.name),
        "functions": tests
    }

    if output_file:
        output_file.write_text(json.dumps(result, indent=2))
        print(f"Written to {output_file}", file=sys.stderr)
    else:
        print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main()
