//! IFC STEP File Parser
//!
//! Parses IFC files using the STEP format (ISO 10303-21).
//! Uses nom parser combinators for efficient parsing.

use super::entities::{EntityId, IfcEntity, IfcValue};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1, multispace0, one_of},
    combinator::{map, opt, recognize},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use std::collections::HashMap;

/// Parse result type
type ParseResult<'a, T> = IResult<&'a str, T>;

/// IFC File structure
#[derive(Debug, Clone)]
pub struct IfcFile {
    pub header: IfcHeader,
    pub entities: HashMap<EntityId, IfcEntity>,
}

/// IFC Header information
#[derive(Debug, Clone)]
pub struct IfcHeader {
    pub file_description: Vec<String>,
    pub file_name: String,
    pub time_stamp: String,
    pub author: Vec<String>,
    pub organization: Vec<String>,
    pub preprocessor_version: String,
    pub originating_system: String,
    pub authorization: String,
}

impl IfcFile {
    /// Create a new empty IFC file
    pub fn new() -> Self {
        Self {
            header: IfcHeader::default(),
            entities: HashMap::new(),
        }
    }

    /// Parse IFC file from string
    pub fn parse(input: &str) -> Result<Self, String> {
        // Normalize line endings (handle both Windows \r\n and Unix \n)
        let normalized = input.replace("\r\n", "\n");

        match parse_ifc_file(&normalized) {
            Ok((_, ifc_file)) => Ok(ifc_file),
            Err(e) => Err(format!("Failed to parse IFC file: {:?}", e)),
        }
    }

    /// Get entity by ID
    pub fn get_entity(&self, id: EntityId) -> Option<&IfcEntity> {
        self.entities.get(&id)
    }

    /// Get all entities of a specific type
    pub fn get_entities_by_type(&self, entity_type: &str) -> Vec<&IfcEntity> {
        self.entities
            .values()
            .filter(|e| e.entity_type.eq_ignore_ascii_case(entity_type))
            .collect()
    }

    /// Get total entity count
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for IfcHeader {
    fn default() -> Self {
        Self {
            file_description: Vec::new(),
            file_name: String::new(),
            time_stamp: String::new(),
            author: Vec::new(),
            organization: Vec::new(),
            preprocessor_version: String::new(),
            originating_system: String::new(),
            authorization: String::new(),
        }
    }
}

/// Parse complete IFC file
fn parse_ifc_file(input: &str) -> ParseResult<IfcFile> {
    let (input, _) = parse_iso_header(input)?;
    let (input, header) = parse_header_section(input)?;
    let (input, entities) = parse_data_section(input)?;
    let (input, _) = parse_iso_footer(input)?;

    Ok((
        input,
        IfcFile {
            header,
            entities: entities.into_iter().map(|e| (e.id, e)).collect(),
        },
    ))
}

/// Parse ISO 10303-21 header
fn parse_iso_header(input: &str) -> ParseResult<()> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("ISO-10303-21;")(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, ()))
}

/// Parse ISO 10303-21 footer
fn parse_iso_footer(input: &str) -> ParseResult<()> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("END-ISO-10303-21;")(input)?;
    Ok((input, ()))
}

/// Parse HEADER section
fn parse_header_section(input: &str) -> ParseResult<IfcHeader> {
    let (input, _) = tag("HEADER;")(input)?;
    let (input, _) = multispace0(input)?;

    // For now, skip header parsing and use default
    // TODO: Implement full header parsing
    let (input, _) = take_until("ENDSEC;")(input)?;
    let (input, _) = tag("ENDSEC;")(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, IfcHeader::default()))
}

/// Parse DATA section
fn parse_data_section(input: &str) -> ParseResult<Vec<IfcEntity>> {
    let (input, _) = tag("DATA;")(input)?;
    let (input, _) = multispace0(input)?;

    let (input, entities) = many0(parse_entity_instance)(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = tag("ENDSEC;")(input)?;

    Ok((input, entities))
}

/// Parse a single entity instance: #123=IFCWALL(...);
fn parse_entity_instance(input: &str) -> ParseResult<IfcEntity> {
    let (input, _) = multispace0(input)?;
    let (input, id) = parse_entity_id(input)?;
    let (input, _) = char('=')(input)?;
    let (input, entity_type) = parse_entity_type(input)?;
    let (input, attributes) = parse_attribute_list(input)?;
    let (input, _) = char(';')(input)?;
    let (input, _) = multispace0(input)?;

    Ok((
        input,
        IfcEntity {
            id,
            entity_type,
            attributes,
        },
    ))
}

/// Parse entity ID: #123
fn parse_entity_id(input: &str) -> ParseResult<EntityId> {
    let (input, _) = char('#')(input)?;
    let (input, id_str) = digit1(input)?;
    let id = id_str.parse::<EntityId>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, id))
}

/// Parse entity type: IFCWALL
fn parse_entity_type(input: &str) -> ParseResult<String> {
    let (input, type_str) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    Ok((input, type_str.to_uppercase()))
}

/// Parse attribute list: (attr1,attr2,attr3)
fn parse_attribute_list(input: &str) -> ParseResult<Vec<IfcValue>> {
    delimited(
        char('('),
        separated_list0(char(','), parse_value),
        char(')'),
    )(input)
}

/// Parse a single value
fn parse_value(input: &str) -> ParseResult<IfcValue> {
    let (input, _) = multispace0(input)?;
    let result = alt((
        map(tag("$"), |_| IfcValue::Null),
        map(parse_entity_ref, IfcValue::EntityRef),
        map(parse_string, IfcValue::String),
        map(parse_float, IfcValue::Real),
        map(parse_integer, IfcValue::Integer),
        map(parse_boolean, IfcValue::Boolean), // Must come before parse_enum
        map(parse_enum, IfcValue::Enum),
        map(parse_list, IfcValue::List),
    ))(input)?;
    let (input, _) = multispace0(input)?;
    Ok(result)
}

/// Parse entity reference: #123
fn parse_entity_ref(input: &str) -> ParseResult<EntityId> {
    parse_entity_id(input)
}

/// Parse string: 'hello'
fn parse_string(input: &str) -> ParseResult<String> {
    let (input, _) = char('\'')(input)?;
    let (input, content) = take_while(|c| c != '\'')(input)?;
    let (input, _) = char('\'')(input)?;
    Ok((input, content.to_string()))
}

/// Parse integer: 123 or -456
fn parse_integer(input: &str) -> ParseResult<i64> {
    let (input, sign) = opt(one_of("+-"))(input)?;
    let (input, digits) = digit1(input)?;

    // Make sure next char is not '.' (which would make it a float)
    if let Ok((_, '.')) = char::<_, nom::error::Error<_>>('.')(input) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    let mut value = digits.parse::<i64>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;

    if sign == Some('-') {
        value = -value;
    }

    Ok((input, value))
}

/// Parse float: 123.456 or -0.5 or 1.5E-3
fn parse_float(input: &str) -> ParseResult<f64> {
    let (input, sign) = opt(one_of("+-"))(input)?;
    let (input, num_str) = recognize(tuple((
        digit1,
        opt(tuple((char('.'), digit1))),
        opt(tuple((one_of("eE"), opt(one_of("+-")), digit1))),
    )))(input)?;

    let mut value = num_str.parse::<f64>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
    })?;

    if sign == Some('-') {
        value = -value;
    }

    Ok((input, value))
}

/// Parse enumeration: .ENUMVALUE.
fn parse_enum(input: &str) -> ParseResult<String> {
    let (input, _) = char('.')(input)?;
    let (input, value) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = char('.')(input)?;
    Ok((input, value.to_uppercase()))
}

/// Parse boolean: .T. or .F.
fn parse_boolean(input: &str) -> ParseResult<bool> {
    alt((
        map(tag(".T."), |_| true),
        map(tag(".F."), |_| false),
        map(tag(".TRUE."), |_| true),
        map(tag(".FALSE."), |_| false),
    ))(input)
}

/// Parse list: (val1,val2,val3)
fn parse_list(input: &str) -> ParseResult<Vec<IfcValue>> {
    delimited(
        char('('),
        separated_list0(char(','), parse_value),
        char(')'),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity_id() {
        assert_eq!(parse_entity_id("#123"), Ok(("", 123)));
        assert_eq!(parse_entity_id("#1"), Ok(("", 1)));
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            parse_string("'hello'"),
            Ok(("", "hello".to_string()))
        );
        assert_eq!(
            parse_string("'IFC File'"),
            Ok(("", "IFC File".to_string()))
        );
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(parse_integer("123"), Ok(("", 123)));
        assert_eq!(parse_integer("-456"), Ok(("", -456)));
        assert_eq!(parse_integer("0"), Ok(("", 0)));
    }

    #[test]
    fn test_parse_float() {
        assert_eq!(parse_float("123.456"), Ok(("", 123.456)));
        assert_eq!(parse_float("-0.5"), Ok(("", -0.5)));
        assert_eq!(parse_float("1.5E-3"), Ok(("", 0.0015)));
    }

    #[test]
    fn test_parse_boolean() {
        assert_eq!(parse_boolean(".T."), Ok(("", true)));
        assert_eq!(parse_boolean(".F."), Ok(("", false)));
    }

    #[test]
    fn test_parse_entity_ref() {
        assert_eq!(parse_entity_ref("#42"), Ok(("", 42)));
    }

    #[test]
    fn test_parse_list() {
        let result = parse_list("(1,2,3)");
        assert!(result.is_ok());
        let (_, list) = result.unwrap();
        assert_eq!(list.len(), 3);
    }
}
