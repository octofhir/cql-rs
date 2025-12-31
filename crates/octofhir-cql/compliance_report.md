# CQL Specification Compliance Report

## Summary

| Metric | Count |
|--------|-------|
| Total Tests | 1693 |
| Passed | 1672 (98.8%) |
| Failed | 5 |
| Skipped | 16 |

## Results by Suite

### CqlNullologicalOperatorsTest

- Passed: 22/22
- Failed: 0
- Skipped: 0

### CqlDateTimeOperatorsTest

- Passed: 317/317
- Failed: 0
- Skipped: 0

### CqlArithmeticFunctionsTest

- Passed: 196/212
- Failed: 0
- Skipped: 16

### CqlQueryTest

- Passed: 12/12
- Failed: 0
- Skipped: 0

### CqlTypeOperatorsTest

- Passed: 35/35
- Failed: 0
- Skipped: 0

### CqlStringOperatorsTest

- Passed: 81/81
- Failed: 0
- Skipped: 0

### CqlAggregateFunctionsTest

- Passed: 50/50
- Failed: 0
- Skipped: 0

### CqlTypesTest

- Passed: 27/28
- Failed: 1
- Skipped: 0

#### Failed Tests

- **Time::TimeUpperBoundMillis**
  - Expected: `Error (Semantic)`
  - Actual: `Error: Parse error: Parse { code: ErrorCode(1), message: "Parse error: ContextError { context: [], cause: None }", expression: "library Test version '1.0'\ndefine Result: @T23:59:59.10000", location: None, context: None }`
  - Error: Parse error: Parse { code: ErrorCode(1), message: "Parse error: ContextError { context: [], cause: None }", expression: "library Test version '1.0'\ndefine Result: @T23:59:59.10000", location: None, context: None }

### CqlConditionalOperatorsTest

- Passed: 9/9
- Failed: 0
- Skipped: 0

### CqlIntervalOperatorsTest

- Passed: 411/412
- Failed: 1
- Skipped: 0

#### Failed Tests

- **In::Issue32Interval**
  - Expected: `true`
  - Actual: ``
  - Error: Evaluation error: UnsupportedOperator { operator: "Starts", types: "Interval, Quantity" }

### CqlListOperatorsTest

- Passed: 212/212
- Failed: 0
- Skipped: 0

### ValueLiteralsAndSelectors

- Passed: 63/66
- Failed: 3
- Skipped: 0

#### Failed Tests

- **Decimal::Decimal10Pow28ToZeroOneStepDecimalMaxValue**
  - Expected: `9999999999999999999999999999.99999999`
  - Actual: `10000000000000000000000000000.0`
- **Decimal::DecimalPos10Pow28ToZeroOneStepDecimalMaxValue**
  - Expected: `9999999999999999999999999999.99999999`
  - Actual: `10000000000000000000000000000.0`
- **Decimal::DecimalNeg10Pow28ToZeroOneStepDecimalMinValue**
  - Expected: `-9999999999999999999999999999.99999999`
  - Actual: `-10000000000000000000000000000.0`

### CqlLogicalOperatorsTest

- Passed: 39/39
- Failed: 0
- Skipped: 0

### CqlComparisonOperatorsTest

- Passed: 198/198
- Failed: 0
- Skipped: 0

