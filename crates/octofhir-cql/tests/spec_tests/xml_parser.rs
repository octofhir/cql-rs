//! XML Parser for CQFramework Test Format
//!
//! Parses the XML test format used by cqframework/cql-tests repository.
//! The format is shared with FHIRPath tests and defined in testSchema.xsd.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::fs;
use std::path::Path;

/// A test suite containing multiple groups of tests
#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub reference: Option<String>,
    pub capabilities: Vec<Capability>,
    pub notes: Option<String>,
    pub groups: Vec<TestGroup>,
}

/// A group of related tests
#[derive(Debug, Clone)]
pub struct TestGroup {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub reference: Option<String>,
    pub capabilities: Vec<Capability>,
    pub notes: Option<String>,
    pub tests: Vec<TestCase>,
}

/// An individual test case
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub reference: Option<String>,
    pub capabilities: Vec<Capability>,
    pub expression: String,
    pub invalid: Option<InvalidType>,
    pub outputs: Vec<ExpectedOutput>,
    pub notes: Option<String>,
    pub skip_static_check: bool,
    pub ordered: bool,
    pub predicate: bool,
}

/// A capability requirement for a test
#[derive(Debug, Clone)]
pub struct Capability {
    pub code: String,
    pub value: Option<String>,
}

/// Expected output of a test
#[derive(Debug, Clone)]
pub struct ExpectedOutput {
    pub value: String,
    pub output_type: Option<OutputType>,
}

/// Type of expected output
#[derive(Debug, Clone, PartialEq)]
pub enum OutputType {
    Boolean,
    Code,
    Date,
    DateTime,
    Decimal,
    Integer,
    Quantity,
    String,
    Time,
}

/// Type of invalid expression
#[derive(Debug, Clone, PartialEq)]
pub enum InvalidType {
    False,    // Not invalid (success)
    Syntax,   // Syntax error
    Semantic, // Semantic error
    Execution,// Execution error
    True,     // Runtime error (generic)
}

impl OutputType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "boolean" => Some(OutputType::Boolean),
            "code" => Some(OutputType::Code),
            "date" => Some(OutputType::Date),
            "datetime" => Some(OutputType::DateTime),
            "decimal" => Some(OutputType::Decimal),
            "integer" => Some(OutputType::Integer),
            "quantity" => Some(OutputType::Quantity),
            "string" => Some(OutputType::String),
            "time" => Some(OutputType::Time),
            _ => None,
        }
    }
}

impl InvalidType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "false" => Some(InvalidType::False),
            "syntax" => Some(InvalidType::Syntax),
            "semantic" => Some(InvalidType::Semantic),
            "execution" => Some(InvalidType::Execution),
            "true" => Some(InvalidType::True),
            _ => None,
        }
    }
}

/// Parse a test suite from an XML file
pub fn parse_test_file(path: &Path) -> Result<TestSuite, ParseError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ParseError::IoError(e.to_string()))?;
    parse_test_xml(&content)
}

/// Parse a test suite from XML string
pub fn parse_test_xml(xml: &str) -> Result<TestSuite, ParseError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut suite = TestSuite {
        name: String::new(),
        version: None,
        description: None,
        reference: None,
        capabilities: Vec::new(),
        notes: None,
        groups: Vec::new(),
    };

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"tests" => {
                parse_tests_attributes(e, &mut suite)?;
                parse_tests_content(&mut reader, &mut suite)?;
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
        }
        buf.clear();
    }

    Ok(suite)
}

fn parse_tests_attributes(e: &BytesStart, suite: &mut TestSuite) -> Result<(), ParseError> {
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
        let value = attr.unescape_value()
            .map_err(|e| ParseError::XmlError(e.to_string()))?
            .to_string();

        match key {
            "name" => suite.name = value,
            "version" => suite.version = Some(value),
            "description" => suite.description = Some(value),
            "reference" => suite.reference = Some(value),
            _ => {}
        }
    }
    Ok(())
}

fn parse_tests_content(reader: &mut Reader<&[u8]>, suite: &mut TestSuite) -> Result<(), ParseError> {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"capability" => {
                        suite.capabilities.push(parse_capability(e)?);
                    }
                    b"notes" => {
                        suite.notes = Some(read_text_content(reader)?);
                    }
                    b"group" => {
                        suite.groups.push(parse_group(e, reader)?);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"tests" => break,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
        }
        buf.clear();
    }

    Ok(())
}

fn parse_capability(e: &BytesStart) -> Result<Capability, ParseError> {
    let mut cap = Capability {
        code: String::new(),
        value: None,
    };

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
        let value = attr.unescape_value()
            .map_err(|e| ParseError::XmlError(e.to_string()))?
            .to_string();

        match key {
            "code" => cap.code = value,
            "value" => cap.value = Some(value),
            _ => {}
        }
    }

    Ok(cap)
}

fn parse_group(e: &BytesStart, reader: &mut Reader<&[u8]>) -> Result<TestGroup, ParseError> {
    let mut group = TestGroup {
        name: String::new(),
        version: None,
        description: None,
        reference: None,
        capabilities: Vec::new(),
        notes: None,
        tests: Vec::new(),
    };

    // Parse attributes
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
        let value = attr.unescape_value()
            .map_err(|e| ParseError::XmlError(e.to_string()))?
            .to_string();

        match key {
            "name" => group.name = value,
            "version" => group.version = Some(value),
            "description" => group.description = Some(value),
            "reference" => group.reference = Some(value),
            _ => {}
        }
    }

    // Parse content
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"capability" => {
                        group.capabilities.push(parse_capability(e)?);
                    }
                    b"notes" => {
                        group.notes = Some(read_text_content(reader)?);
                    }
                    b"test" => {
                        group.tests.push(parse_test(e, reader)?);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"capability" => {
                group.capabilities.push(parse_capability(e)?);
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"group" => break,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
        }
        buf.clear();
    }

    Ok(group)
}

fn parse_test(e: &BytesStart, reader: &mut Reader<&[u8]>) -> Result<TestCase, ParseError> {
    let mut test = TestCase {
        name: String::new(),
        version: None,
        description: None,
        reference: None,
        capabilities: Vec::new(),
        expression: String::new(),
        invalid: None,
        outputs: Vec::new(),
        notes: None,
        skip_static_check: false,
        ordered: false,
        predicate: false,
    };

    // Parse attributes
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
        let value = attr.unescape_value()
            .map_err(|e| ParseError::XmlError(e.to_string()))?
            .to_string();

        match key {
            "name" => test.name = value,
            "version" => test.version = Some(value),
            "description" => test.description = Some(value),
            "reference" => test.reference = Some(value),
            "skipStaticCheck" => test.skip_static_check = value == "true",
            "ordered" => test.ordered = value == "true",
            "predicate" => test.predicate = value == "true",
            _ => {}
        }
    }

    // Parse content
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"capability" => {
                        test.capabilities.push(parse_capability(e)?);
                    }
                    b"expression" => {
                        // Check for invalid attribute
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref())
                                .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
                            if key == "invalid" {
                                let value = attr.unescape_value()
                                    .map_err(|e| ParseError::XmlError(e.to_string()))?
                                    .to_string();
                                test.invalid = InvalidType::from_str(&value);
                            }
                        }
                        test.expression = read_text_content(reader)?;
                    }
                    b"output" => {
                        let output = parse_output(e, reader)?;
                        test.outputs.push(output);
                    }
                    b"notes" => {
                        test.notes = Some(read_text_content(reader)?);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"capability" => {
                        test.capabilities.push(parse_capability(e)?);
                    }
                    b"output" => {
                        // Empty output means null/empty result
                        test.outputs.push(ExpectedOutput {
                            value: String::new(),
                            output_type: None,
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"test" => break,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
        }
        buf.clear();
    }

    Ok(test)
}

fn parse_output(e: &BytesStart, reader: &mut Reader<&[u8]>) -> Result<ExpectedOutput, ParseError> {
    let mut output = ExpectedOutput {
        value: String::new(),
        output_type: None,
    };

    // Parse type attribute
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
        if key == "type" {
            let value = attr.unescape_value()
                .map_err(|e| ParseError::XmlError(e.to_string()))?
                .to_string();
            output.output_type = OutputType::from_str(&value);
        }
    }

    output.value = read_text_content(reader)?;
    Ok(output)
}

fn read_text_content(reader: &mut Reader<&[u8]>) -> Result<String, ParseError> {
    let mut buf = Vec::new();
    let mut text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                text = e.decode()
                    .map_err(|e| ParseError::XmlError(e.to_string()))?
                    .to_string();
            }
            Ok(Event::CData(e)) => {
                text = String::from_utf8(e.into_inner().to_vec())
                    .map_err(|e| ParseError::Utf8Error(e.to_string()))?;
            }
            Ok(Event::End(_)) => break,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
        }
        buf.clear();
    }

    Ok(text)
}

/// Error type for parsing
#[derive(Debug)]
pub enum ParseError {
    IoError(String),
    XmlError(String),
    Utf8Error(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::IoError(e) => write!(f, "IO error: {}", e),
            ParseError::XmlError(e) => write!(f, "XML error: {}", e),
            ParseError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_test() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tests name="CqlArithmeticFunctionsTest" version="1.0">
    <group name="Abs">
        <test name="Abs0">
            <expression>Abs(0)</expression>
            <output>0</output>
        </test>
        <test name="AbsNeg1">
            <expression>Abs(-1)</expression>
            <output>1</output>
        </test>
    </group>
</tests>"#;

        let suite = parse_test_xml(xml).unwrap();
        assert_eq!(suite.name, "CqlArithmeticFunctionsTest");
        assert_eq!(suite.groups.len(), 1);
        assert_eq!(suite.groups[0].name, "Abs");
        assert_eq!(suite.groups[0].tests.len(), 2);
        assert_eq!(suite.groups[0].tests[0].name, "Abs0");
        assert_eq!(suite.groups[0].tests[0].expression, "Abs(0)");
        assert_eq!(suite.groups[0].tests[0].outputs[0].value, "0");
    }

    #[test]
    fn test_parse_test_with_capability() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tests name="Test" version="1.0">
    <group name="Group1">
        <test name="Test1">
            <capability code="ucum-unit-conversion"/>
            <expression>Abs(-1.0'cm')</expression>
            <output>1.0'cm'</output>
        </test>
    </group>
</tests>"#;

        let suite = parse_test_xml(xml).unwrap();
        assert_eq!(suite.groups[0].tests[0].capabilities.len(), 1);
        assert_eq!(suite.groups[0].tests[0].capabilities[0].code, "ucum-unit-conversion");
    }

    #[test]
    fn test_parse_invalid_expression() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tests name="Test" version="1.0">
    <group name="Group1">
        <test name="Test1">
            <expression invalid="true">Exp(1000)</expression>
        </test>
    </group>
</tests>"#;

        let suite = parse_test_xml(xml).unwrap();
        assert_eq!(suite.groups[0].tests[0].invalid, Some(InvalidType::True));
    }
}
