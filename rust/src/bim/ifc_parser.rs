//! IFC STEP File Parser
//!
//! Parses IFC files using the STEP format (ISO 10303-21).
//! Uses nom parser combinators for efficient parsing.

use super::entities::{EntityId, IfcEntity, IfcSchemaVersion, IfcValue};
use rayon::prelude::*;
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
use std::io::{BufRead, BufReader, Read};

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
        // Only allocate a normalized copy if the file actually contains \r
        // This avoids doubling memory for Unix-line-ending files
        let has_cr = input.contains('\r');
        let normalized;
        let parse_input = if has_cr {
            normalized = input.replace("\r\n", "\n");
            normalized.as_str()
        } else {
            input
        };

        match parse_ifc_file(parse_input) {
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

    let mut entity_map = HashMap::with_capacity(entities.len());
    for e in entities {
        entity_map.insert(e.id, e);
    }

    Ok((
        input,
        IfcFile {
            header,
            entities: entity_map,
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

/// Parse a single entity instance: #123=IFCWALL(...); or #123 = IFCWALL(...);
fn parse_entity_instance(input: &str) -> ParseResult<IfcEntity> {
    let (input, _) = multispace0(input)?;
    let (input, id) = parse_entity_id(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
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
        map(tag("*"), |_| IfcValue::Null), // Derived/inherited marker
        map(parse_entity_ref, IfcValue::EntityRef),
        map(parse_string, IfcValue::String),
        parse_typed_value, // TYPENAME(value) - must come before float/int
        map(parse_float, IfcValue::Real),
        map(parse_integer, IfcValue::Integer),
        map(parse_boolean, IfcValue::Boolean), // Must come before parse_enum
        map(parse_enum, IfcValue::Enum),
        map(parse_list, IfcValue::List),
    ))(input)?;
    let (_input, _) = multispace0(input)?;
    Ok(result)
}

/// Parse typed value: IFCMASSMEASURE(1826.69), IFCLABEL('text'), etc.
/// These are IFC STEP "typed values" where a type name wraps a value.
fn parse_typed_value(input: &str) -> ParseResult<IfcValue> {
    // Must start with uppercase letter (IFC type names are always uppercase)
    if !input.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    let (input, _type_name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = char('(')(input)?;
    let (input, values) = separated_list0(char(','), parse_value)(input)?;
    let (input, _) = char(')')(input)?;
    // Unwrap single-value typed values, keep multi-value as list
    let value = if values.len() == 1 {
        values.into_iter().next().unwrap()
    } else {
        IfcValue::List(values)
    };
    Ok((input, value))
}

/// Parse entity reference: #123
fn parse_entity_ref(input: &str) -> ParseResult<EntityId> {
    parse_entity_id(input)
}

/// Parse string: 'hello' or 'it''s' ('' is escaped single quote in STEP)
fn parse_string(input: &str) -> ParseResult<String> {
    let (input, _) = char('\'')(input)?;
    let mut result = std::string::String::new();
    let mut remaining = input;
    loop {
        let (input, content) = take_while(|c| c != '\'')(remaining)?;
        result.push_str(content);
        let (input, _) = char('\'')(input)?;
        // Check for escaped quote ('')
        if input.starts_with('\'') {
            result.push('\'');
            remaining = &input[1..]; // skip the second quote
        } else {
            return Ok((input, result));
        }
    }
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

/// Parse float: 123.456 or -0.5 or 1.5E-3 or 1200. (trailing dot)
fn parse_float(input: &str) -> ParseResult<f64> {
    let (input, sign) = opt(one_of("+-"))(input)?;
    let (input, num_str) = recognize(tuple((
        digit1,
        // Allow trailing dot with optional digits (e.g. "1200." or "123.456")
        opt(tuple((char('.'), opt(digit1)))),
        opt(tuple((one_of("eE"), opt(one_of("+-")), digit1))),
    )))(input)?;

    // Trailing dot like "1200." needs to parse as float — use Cow to avoid allocation
    use std::borrow::Cow;
    let num_clean: Cow<str> = if num_str.ends_with('.') {
        Cow::Owned(format!("{}0", num_str))
    } else {
        Cow::Borrowed(num_str)
    };

    let mut value = num_clean.parse::<f64>().map_err(|_| {
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

// ========================================================================
// IFC Georeferencing Support
// ========================================================================

/// Georeferencing data parsed from IFC IFCMAPCONVERSION and IFCPROJECTEDCRS entities.
#[derive(Debug, Clone)]
pub struct MapConversion {
    pub eastings: f64,
    pub northings: f64,
    pub orthogonal_height: f64,
    pub x_axis_abscissa: f64,
    pub x_axis_ordinate: f64,
    pub scale: f64,
    pub crs_name: String,
    pub crs_description: String,
    pub geodetic_datum: String,
    pub vertical_datum: String,
    pub map_projection: String,
    pub map_zone: String,
}

impl Default for MapConversion {
    fn default() -> Self {
        Self {
            eastings: 0.0,
            northings: 0.0,
            orthogonal_height: 0.0,
            x_axis_abscissa: 1.0,
            x_axis_ordinate: 0.0,
            scale: 1.0,
            crs_name: String::new(),
            crs_description: String::new(),
            geodetic_datum: String::new(),
            vertical_datum: String::new(),
            map_projection: String::new(),
            map_zone: String::new(),
        }
    }
}

/// Extract IFCMAPCONVERSION georeferencing data from parsed IFC entities.
///
/// Searches for IFCMAPCONVERSION entities and their associated IFCPROJECTEDCRS
/// to build a complete georeferencing description.
///
/// IFCMAPCONVERSION attributes:
///   0: SourceCRS (ref), 1: TargetCRS (ref), 2: Eastings, 3: Northings,
///   4: OrthogonalHeight, 5: XAxisAbscissa (opt), 6: XAxisOrdinate (opt), 7: Scale (opt)
///
/// IFCPROJECTEDCRS attributes:
///   0: Name, 1: Description (opt), 2: GeodeticDatum (opt), 3: VerticalDatum (opt),
///   4: MapProjection (opt), 5: MapZone (opt), 6: MapUnit (opt)
pub fn extract_map_conversion(entities: &HashMap<i32, IfcEntity>) -> Option<MapConversion> {
    // Find IFCMAPCONVERSION entity
    let map_conv_entity = entities
        .values()
        .find(|e| e.entity_type.eq_ignore_ascii_case("IFCMAPCONVERSION"))?;

    let mut result = MapConversion {
        eastings: map_conv_entity.get_real(2).unwrap_or(0.0),
        northings: map_conv_entity.get_real(3).unwrap_or(0.0),
        orthogonal_height: map_conv_entity.get_real(4).unwrap_or(0.0),
        x_axis_abscissa: map_conv_entity.get_real(5).unwrap_or(1.0),
        x_axis_ordinate: map_conv_entity.get_real(6).unwrap_or(0.0),
        scale: map_conv_entity.get_real(7).unwrap_or(1.0),
        ..Default::default()
    };

    // Resolve TargetCRS (attr 1) to get IFCPROJECTEDCRS data
    if let Some(target_crs_id) = map_conv_entity.get_entity_ref(1) {
        if let Some(crs_entity) = entities.get(&target_crs_id) {
            if crs_entity
                .entity_type
                .eq_ignore_ascii_case("IFCPROJECTEDCRS")
            {
                result.crs_name = crs_entity.get_string(0).unwrap_or_default();
                result.crs_description = crs_entity.get_string(1).unwrap_or_default();
                result.geodetic_datum = crs_entity.get_string(2).unwrap_or_default();
                result.vertical_datum = crs_entity.get_string(3).unwrap_or_default();
                result.map_projection = crs_entity.get_string(4).unwrap_or_default();
                result.map_zone = crs_entity.get_string(5).unwrap_or_default();
            }
        }
    }

    Some(result)
}

// ========================================================================
// IFC Unit Detection Support
// ========================================================================

/// Detected unit factors from the IFC file.
#[derive(Debug, Clone)]
pub struct IfcUnitFactors {
    /// Multiply IFC length values by this to get meters
    pub length_factor: f64,
    /// Multiply IFC area values by this to get square meters
    pub area_factor: f64,
    /// Multiply IFC volume values by this to get cubic meters
    pub volume_factor: f64,
    /// Human-readable description of the detected units
    pub description: String,
}

impl Default for IfcUnitFactors {
    fn default() -> Self {
        Self {
            length_factor: 1.0,
            area_factor: 1.0,
            volume_factor: 1.0,
            description: "Default (meters)".to_string(),
        }
    }
}

/// Detect units from IFCUNITASSIGNMENT in the IFC entity map.
///
/// Searches for IFCUNITASSIGNMENT, then resolves each unit in its Units list
/// to determine the length, area, and volume conversion factors to SI (meters).
pub fn detect_ifc_units(entities: &HashMap<i32, IfcEntity>) -> IfcUnitFactors {
    let mut factors = IfcUnitFactors::default();
    let mut length_label = String::from("m");
    let mut area_label = String::from("m\u{b2}");
    let mut volume_label = String::from("m\u{b3}");

    // Find IFCUNITASSIGNMENT
    let unit_assignment = match entities
        .values()
        .find(|e| e.entity_type.eq_ignore_ascii_case("IFCUNITASSIGNMENT"))
    {
        Some(e) => e,
        None => return factors,
    };

    // Units (attr 0) = list of unit entity refs
    let unit_list = match unit_assignment.get_list(0) {
        Some(list) => list,
        None => return factors,
    };

    for val in unit_list {
        let unit_id = match val {
            IfcValue::EntityRef(id) => *id,
            _ => continue,
        };

        let unit_entity = match entities.get(&unit_id) {
            Some(e) => e,
            None => continue,
        };

        let etype = unit_entity.entity_type.as_str();

        if etype.eq_ignore_ascii_case("IFCSIUNIT") {
            // IFCSIUNIT(*, UnitType, Prefix, Name)
            // Attr indices: 0=Dimensions(usually *), 1=UnitType, 2=Prefix, 3=Name
            let unit_type = match unit_entity.get_attr(1) {
                Some(IfcValue::Enum(s)) => s.clone(),
                _ => continue,
            };

            let prefix = match unit_entity.get_attr(2) {
                Some(IfcValue::Enum(s)) => Some(s.clone()),
                _ => None,
            };

            let prefix_factor = match prefix.as_deref() {
                Some("MILLI") => 0.001,
                Some("CENTI") => 0.01,
                Some("DECI") => 0.1,
                Some("KILO") => 1000.0,
                Some("HECTO") => 100.0,
                Some("DECA") => 10.0,
                Some("MICRO") => 0.000001,
                _ => 1.0, // No prefix or unknown prefix
            };

            let prefix_label = match prefix.as_deref() {
                Some("MILLI") => "mm",
                Some("CENTI") => "cm",
                Some("DECI") => "dm",
                Some("KILO") => "km",
                _ => "m",
            };

            match unit_type.as_str() {
                "LENGTHUNIT" => {
                    factors.length_factor = prefix_factor;
                    length_label = prefix_label.to_string();
                }
                "AREAUNIT" => {
                    factors.area_factor = prefix_factor * prefix_factor;
                    area_label = format!("{}\u{b2}", prefix_label);
                }
                "VOLUMEUNIT" => {
                    factors.volume_factor = prefix_factor * prefix_factor * prefix_factor;
                    volume_label = format!("{}\u{b3}", prefix_label);
                }
                _ => {}
            }
        } else if etype.eq_ignore_ascii_case("IFCCONVERSIONBASEDUNIT") {
            // IFCCONVERSIONBASEDUNIT(Dimensions, UnitType, Name, ConversionFactor)
            let unit_type = match unit_entity.get_attr(1) {
                Some(IfcValue::Enum(s)) => s.clone(),
                _ => continue,
            };

            let unit_name = unit_entity.get_string(2).unwrap_or_default();

            // Resolve ConversionFactor (attr 3) -> IFCMEASUREWITHUNIT
            let conv_factor_id = match unit_entity.get_entity_ref(3) {
                Some(id) => id,
                None => continue,
            };

            let factor_value = resolve_conversion_factor(conv_factor_id, entities);

            match unit_type.as_str() {
                "LENGTHUNIT" => {
                    factors.length_factor = factor_value;
                    length_label = unit_name;
                }
                "AREAUNIT" => {
                    factors.area_factor = factor_value;
                    area_label = unit_name;
                }
                "VOLUMEUNIT" => {
                    factors.volume_factor = factor_value;
                    volume_label = unit_name;
                }
                _ => {}
            }
        }
    }

    // If area or volume factors were not explicitly set, derive from length
    // This handles cases where only LENGTHUNIT is defined
    if (factors.area_factor - 1.0).abs() < 1e-12
        && (factors.length_factor - 1.0).abs() > 1e-12
    {
        factors.area_factor = factors.length_factor * factors.length_factor;
        area_label = format!("{}\u{b2}", length_label);
    }
    if (factors.volume_factor - 1.0).abs() < 1e-12
        && (factors.length_factor - 1.0).abs() > 1e-12
    {
        factors.volume_factor = factors.length_factor * factors.length_factor * factors.length_factor;
        volume_label = format!("{}\u{b3}", length_label);
    }

    factors.description = format!(
        "Length: {}, Area: {}, Volume: {}",
        length_label, area_label, volume_label
    );

    factors
}

/// Resolve an IFCMEASUREWITHUNIT to get the numeric conversion factor.
/// IFCMEASUREWITHUNIT(ValueComponent, UnitComponent)
fn resolve_conversion_factor(id: i32, entities: &HashMap<i32, IfcEntity>) -> f64 {
    let entity = match entities.get(&id) {
        Some(e) => e,
        None => return 1.0,
    };

    if !entity
        .entity_type
        .eq_ignore_ascii_case("IFCMEASUREWITHUNIT")
    {
        return 1.0;
    }

    // ValueComponent (attr 0) - the numeric factor
    let value = match entity.get_real(0) {
        Some(v) => v,
        None => return 1.0,
    };

    // UnitComponent (attr 1) may reference an IFCSIUNIT with its own prefix
    if let Some(unit_ref_id) = entity.get_entity_ref(1) {
        if let Some(unit_entity) = entities.get(&unit_ref_id) {
            if unit_entity
                .entity_type
                .eq_ignore_ascii_case("IFCSIUNIT")
            {
                let prefix = match unit_entity.get_attr(2) {
                    Some(IfcValue::Enum(s)) => Some(s.clone()),
                    _ => None,
                };
                let prefix_factor = match prefix.as_deref() {
                    Some("MILLI") => 0.001,
                    Some("CENTI") => 0.01,
                    Some("DECI") => 0.1,
                    Some("KILO") => 1000.0,
                    _ => 1.0,
                };
                return value * prefix_factor;
            }
        }
    }

    value
}

// ========================================================================
// IFC Schema Version Detection
// ========================================================================

/// Detect IFC schema version from the FILE_SCHEMA header line.
///
/// Parses the FILE_SCHEMA line in the IFC HEADER section:
/// - `FILE_SCHEMA(('IFC2X3'));` -> Ifc2x3
/// - `FILE_SCHEMA(('IFC4'));` -> Ifc4
/// - `FILE_SCHEMA(('IFC4X3'));` or `FILE_SCHEMA(('IFC4X3_ADD2'));` -> Ifc4x3
/// - Unknown -> Unknown(schema_string)
pub fn detect_ifc_schema(content: &str) -> IfcSchemaVersion {
    // Search for FILE_SCHEMA line (case-insensitive)
    for line in content.lines() {
        let trimmed = line.trim().to_uppercase();
        if trimmed.starts_with("FILE_SCHEMA") {
            // Extract schema string between quotes
            if let Some(start) = trimmed.find('\'') {
                if let Some(end) = trimmed[start + 1..].find('\'') {
                    let schema = &trimmed[start + 1..start + 1 + end];
                    return match_schema_version(schema);
                }
            }
        }
    }

    IfcSchemaVersion::Unknown("".to_string())
}

/// Match a schema string to an IfcSchemaVersion.
fn match_schema_version(schema: &str) -> IfcSchemaVersion {
    let upper = schema.to_uppercase();

    if upper == "IFC2X3" || upper.starts_with("IFC2X3_") || upper == "IFC2X_PLATFORM" {
        IfcSchemaVersion::Ifc2x3
    } else if upper.starts_with("IFC4X3") {
        // IFC4X3, IFC4X3_ADD1, IFC4X3_ADD2, etc.
        IfcSchemaVersion::Ifc4x3
    } else if upper == "IFC4" || upper.starts_with("IFC4_") || upper == "IFC4X1" || upper == "IFC4X2" {
        // IFC4, IFC4_ADD1, IFC4_ADD2, IFC4X1, IFC4X2
        IfcSchemaVersion::Ifc4
    } else {
        IfcSchemaVersion::Unknown(schema.to_string())
    }
}

/// Map IFC2x3 entity names to their IFC4 equivalents.
///
/// Most entity names are the same across versions, but some were renamed
/// or reorganized in IFC4/IFC4x3. This function normalizes names to the
/// IFC4 canonical form.
pub fn normalize_entity_name(name: &str) -> &str {
    let upper = name.trim();
    // Most entity names are shared across versions, only a few differ.
    // IFC4x3 introduced IFCFACILITY as a generalization of IFCBUILDING,
    // but IFCBUILDING is still valid. We normalize newer names back to IFC4.
    match upper {
        // IFC4x3 additions that map back to IFC4 equivalents
        "IFCFACILITY" => "IFCBUILDING",
        "IFCFACILITYPART" => "IFCBUILDINGSTOREY",
        "IFCROAD" => "IFCBUILDING",
        "IFCRAILWAY" => "IFCBUILDING",
        "IFCBRIDGE" => "IFCBUILDING",
        "IFCMARINEFACILITY" => "IFCBUILDING",
        // IFC2x3 names that were changed in IFC4
        "IFCRELASSOCIATESPROFILEPROPERTIES" => "IFCRELASSOCIATESMATERIAL",
        "IFCENERGYCONVERSIONDEVICE" => "IFCENERGYCONVERSIONDEVICE",
        // These remain the same across versions
        _ => name,
    }
}

// ========================================================================
// Streaming, Parallel, and Partial IFC Parsing
// ========================================================================

/// Header information extracted during streaming parse.
#[derive(Debug, Clone, Default)]
pub struct HeaderInfo {
    pub description: String,
    pub file_name: String,
    pub schema: String,
}

/// Summary of an IFC file obtained by fast scanning without full attribute parsing.
#[derive(Debug, Clone)]
pub struct IfcSummary {
    pub schema: String,
    pub entity_type_counts: HashMap<String, usize>,
    pub total_entities: usize,
    /// Count of product/element entities (walls, slabs, columns, etc.)
    pub estimated_elements: usize,
    /// True if any geometric representation entities are found
    pub has_geometry: bool,
}

/// Known IFC product/element type prefixes used for element counting in summaries.
const PRODUCT_TYPES: &[&str] = &[
    "IFCWALL", "IFCWALLSTANDARDCASE", "IFCSLAB", "IFCCOLUMN", "IFCBEAM",
    "IFCDOOR", "IFCWINDOW", "IFCROOF", "IFCSTAIR", "IFCFURNISHINGELEMENT",
    "IFCFOOTING", "IFCPILE", "IFCRAILING", "IFCRAMP", "IFCSTAIRFLIGHT",
    "IFCPLATE", "IFCMEMBER", "IFCCURTAINWALL", "IFCBUILDINGPROXY",
    "IFCBUILDINGELEMENTPROXY", "IFCFLOWSEGMENT", "IFCFLOWTERMINAL",
    "IFCFLOWFITTING", "IFCPIPESEGMENT", "IFCDUCTSEGMENT",
    "IFCCABLECARRIERSEGMENT", "IFCCOVERING", "IFCOPENINGELEMENT",
    "IFCSPACE",
];

/// Geometry representation entity types used to detect if a model has geometry.
const GEOMETRY_TYPES: &[&str] = &[
    "IFCSHAPEREPRESENTATION", "IFCPRODUCTDEFINITIONSHAPE",
    "IFCEXTRUDEDAREASOLID", "IFCBOOLEANCLIPPINGRESULT",
    "IFCFACETEDBREP", "IFCFACEBASEDSURFACEMODEL",
    "IFCMAPPEDITEM", "IFCTRIANGULATEDFACESET",
    "IFCPOLYGONALBOUNDEDHALFSPACE",
];

/// Streaming IFC parser that processes entities one at a time.
/// Useful for large files (100MB+) where loading everything into memory is expensive.
pub struct StreamingIfcParser;

impl StreamingIfcParser {
    /// Parse IFC content in streaming fashion (line by line).
    /// This avoids building large intermediate data structures during parsing.
    /// Each line in the DATA section is parsed independently and added to the
    /// result HashMap on the spot.
    pub fn parse_streaming(content: &str) -> Result<IfcFile, String> {
        let normalized;
        let parse_input = if content.contains('\r') {
            normalized = content.replace("\r\n", "\n");
            normalized.as_str()
        } else {
            content
        };

        let (header_section, data_section) = find_data_section(parse_input)
            .ok_or_else(|| "Could not find DATA section in IFC file".to_string())?;

        let header_info = scan_header_info(header_section);
        let _ = header_info; // Header info available for future use

        let mut entities = HashMap::new();

        // Process each line in the data section
        for line in data_section.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "ENDSEC;" {
                continue;
            }
            Self::parse_data_line(trimmed, &mut entities);
        }

        Ok(IfcFile {
            header: IfcHeader::default(),
            entities,
        })
    }

    /// Parse a single DATA line and add to entities map.
    /// Returns true if the line was successfully parsed as an entity.
    fn parse_data_line(line: &str, entities: &mut HashMap<EntityId, IfcEntity>) -> bool {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return false;
        }
        // Use the existing nom-based parser for the entity line
        match parse_entity_instance(trimmed) {
            Ok((_, entity)) => {
                entities.insert(entity.id, entity);
                true
            }
            Err(_) => false,
        }
    }

    /// Extract just the entity IDs and types without parsing full attributes.
    /// Useful for partial loading: first scan types, then parse only needed ones.
    pub fn scan_entity_types(content: &str) -> Vec<(EntityId, String)> {
        let normalized;
        let parse_input = if content.contains('\r') {
            normalized = content.replace("\r\n", "\n");
            normalized.as_str()
        } else {
            content
        };

        let (_header, data_section) = match find_data_section(parse_input) {
            Some(sections) => sections,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for line in data_section.lines() {
            let trimmed = line.trim();
            if let Some((id, type_name)) = extract_id_and_type(trimmed) {
                results.push((id, type_name));
            }
        }
        results
    }
}

/// Parse IFC content using parallel processing for the DATA section.
/// Uses rayon to parse entity lines concurrently across multiple threads.
pub fn parse_parallel(content: &str) -> Result<IfcFile, String> {
    let normalized;
    let parse_input = if content.contains('\r') {
        normalized = content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        content
    };

    let (_header, data_section) = find_data_section(parse_input)
        .ok_or_else(|| "Could not find DATA section in IFC file".to_string())?;

    // Collect lines that look like entity definitions
    let lines: Vec<&str> = data_section
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with('#'))
        .collect();

    // Parse each line in parallel using rayon
    let parsed: Vec<Option<IfcEntity>> = lines
        .par_iter()
        .map(|line| match parse_entity_instance(line) {
            Ok((_, entity)) => Some(entity),
            Err(_) => None,
        })
        .collect();

    // Build the HashMap from the parallel results
    let mut entities = HashMap::with_capacity(parsed.len());
    for maybe_entity in parsed {
        if let Some(entity) = maybe_entity {
            entities.insert(entity.id, entity);
        }
    }

    Ok(IfcFile {
        header: IfcHeader::default(),
        entities,
    })
}

/// Parse IFC content but only extract entities of specific types.
/// Much faster for cases where you only need certain elements (e.g., just walls and slabs).
/// The `type_filter` entries are matched case-insensitively against entity type names.
pub fn parse_filtered(content: &str, type_filter: &[&str]) -> Result<IfcFile, String> {
    let normalized;
    let parse_input = if content.contains('\r') {
        normalized = content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        content
    };

    let (_header, data_section) = find_data_section(parse_input)
        .ok_or_else(|| "Could not find DATA section in IFC file".to_string())?;

    // Build an uppercase set for fast type matching
    let filter_set: Vec<String> = type_filter.iter().map(|t| t.to_uppercase()).collect();

    let mut entities = HashMap::new();

    for line in data_section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }
        // Quick check: extract entity type without full parsing
        if let Some((_id, type_name)) = extract_id_and_type(trimmed) {
            let upper_type = type_name.to_uppercase();
            if filter_set.iter().any(|f| f == &upper_type) {
                // Full parse only for matching types
                if let Ok((_, entity)) = parse_entity_instance(trimmed) {
                    entities.insert(entity.id, entity);
                }
            }
        }
    }

    Ok(IfcFile {
        header: IfcHeader::default(),
        entities,
    })
}

/// Parse IFC content but stop after N entities of each type.
/// Useful for previewing large models without loading everything.
pub fn parse_preview(content: &str, max_per_type: usize) -> Result<IfcFile, String> {
    let normalized;
    let parse_input = if content.contains('\r') {
        normalized = content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        content
    };

    let (_header, data_section) = find_data_section(parse_input)
        .ok_or_else(|| "Could not find DATA section in IFC file".to_string())?;

    let mut entities = HashMap::new();
    let mut type_counts: HashMap<String, usize> = HashMap::new();

    for line in data_section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }

        if let Some((_id, type_name)) = extract_id_and_type(trimmed) {
            let upper_type = type_name.to_uppercase();
            let count = type_counts.entry(upper_type).or_insert(0);
            if *count >= max_per_type {
                continue;
            }
            if let Ok((_, entity)) = parse_entity_instance(trimmed) {
                *type_counts.get_mut(&entity.entity_type).unwrap_or(&mut 0) += 1;
                entities.insert(entity.id, entity);
            }
        }
    }

    Ok(IfcFile {
        header: IfcHeader::default(),
        entities,
    })
}

/// Scan an IFC file and return a summary without full attribute parsing.
/// Provides a fast overview: entity type counts, file schema, and whether geometry exists.
pub fn scan_ifc_summary(content: &str) -> IfcSummary {
    let schema_version = detect_ifc_schema(content);
    let schema = match schema_version {
        IfcSchemaVersion::Ifc2x3 => "IFC2X3".to_string(),
        IfcSchemaVersion::Ifc4 => "IFC4".to_string(),
        IfcSchemaVersion::Ifc4x3 => "IFC4X3".to_string(),
        IfcSchemaVersion::Unknown(s) => s,
    };

    let normalized;
    let parse_input = if content.contains('\r') {
        normalized = content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        content
    };

    let (_header, data_section) = match find_data_section(parse_input) {
        Some(sections) => sections,
        None => {
            return IfcSummary {
                schema,
                entity_type_counts: HashMap::new(),
                total_entities: 0,
                estimated_elements: 0,
                has_geometry: false,
            };
        }
    };

    let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
    let mut total_entities = 0usize;

    for line in data_section.lines() {
        let trimmed = line.trim();
        if let Some((_id, type_name)) = extract_id_and_type(trimmed) {
            let upper = type_name.to_uppercase();
            *entity_type_counts.entry(upper).or_insert(0) += 1;
            total_entities += 1;
        }
    }

    let estimated_elements: usize = PRODUCT_TYPES
        .iter()
        .filter_map(|pt| entity_type_counts.get(*pt))
        .sum();

    let has_geometry = GEOMETRY_TYPES
        .iter()
        .any(|gt| entity_type_counts.contains_key(*gt));

    IfcSummary {
        schema,
        entity_type_counts,
        total_entities,
        estimated_elements,
        has_geometry,
    }
}

// ========================================================================
// Helper functions for streaming/parallel/partial parsing
// ========================================================================

/// Find the DATA section in IFC content.
/// Returns (header_content, data_section_content) where data_section_content
/// is the text between `DATA;` and `ENDSEC;` (exclusive of both markers).
fn find_data_section(content: &str) -> Option<(&str, &str)> {
    // Find "DATA;" marker
    let data_start_marker = "DATA;";
    let data_start_idx = content.find(data_start_marker)?;
    let header = &content[..data_start_idx];
    let after_data = &content[data_start_idx + data_start_marker.len()..];

    // Find the ENDSEC; that closes the DATA section
    let endsec_idx = after_data.find("ENDSEC;")?;
    let data_section = &after_data[..endsec_idx];

    Some((header, data_section))
}

/// Quickly extract entity ID and type name from a DATA line without full parsing.
/// Expects lines like `#123=IFCWALL(...);\n` or `#123 = IFCWALL(...);\n`.
/// Returns (EntityId, type_name_string) or None if the line doesn't match.
fn extract_id_and_type(line: &str) -> Option<(EntityId, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    // Skip '#'
    let rest = &trimmed[1..];

    // Parse digits for entity ID
    let id_end = rest.find(|c: char| !c.is_ascii_digit())?;
    if id_end == 0 {
        return None;
    }
    let id: EntityId = rest[..id_end].parse().ok()?;

    // Skip whitespace and '='
    let after_id = rest[id_end..].trim_start();
    let after_eq = after_id.strip_prefix('=')?;
    let after_eq = after_eq.trim_start();

    // Parse type name (alphanumeric + underscore)
    let type_end = after_eq
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(after_eq.len());
    if type_end == 0 {
        return None;
    }
    let type_name = after_eq[..type_end].to_uppercase();

    Some((id, type_name))
}

/// Scan header section text and extract basic header info.
fn scan_header_info(header: &str) -> HeaderInfo {
    let mut info = HeaderInfo::default();

    for line in header.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();

        if upper.starts_with("FILE_SCHEMA") {
            if let Some(start) = trimmed.find('\'') {
                if let Some(end) = trimmed[start + 1..].find('\'') {
                    info.schema = trimmed[start + 1..start + 1 + end].to_string();
                }
            }
        } else if upper.starts_with("FILE_NAME") {
            if let Some(start) = trimmed.find('\'') {
                if let Some(end) = trimmed[start + 1..].find('\'') {
                    info.file_name = trimmed[start + 1..start + 1 + end].to_string();
                }
            }
        } else if upper.starts_with("FILE_DESCRIPTION") {
            if let Some(start) = trimmed.find('\'') {
                if let Some(end) = trimmed[start + 1..].find('\'') {
                    info.description = trimmed[start + 1..start + 1 + end].to_string();
                }
            }
        }
    }

    info
}

// ========================================================================
// Efficient File Reading and Chunked Parsing
// ========================================================================

/// Memory estimate for parsing a file of a given size.
#[derive(Debug, Clone)]
pub struct ParseMemoryEstimate {
    /// File content in memory (MB)
    pub file_content_mb: f32,
    /// Estimated entity map size (MB)
    pub entity_map_mb: f32,
    /// Estimated tessellation memory (MB)
    pub tessellation_mb: f32,
    /// Total estimated memory (MB)
    pub total_mb: f32,
    /// Recommendation: "direct", "chunked", or "streaming"
    pub recommendation: String,
}

/// Estimate memory needed for parsing a file of given size.
///
/// Heuristics:
/// - File content: 1x file size (string in memory)
/// - Entity map: ~1.5x file size (parsed entities with attribute vectors)
/// - Tessellation: ~0.5x file size (vertex/index buffers for geometry)
pub fn estimate_parse_memory(file_size_bytes: u64) -> ParseMemoryEstimate {
    let file_mb = file_size_bytes as f32 / (1024.0 * 1024.0);
    let entity_map_mb = file_mb * 1.5;
    let tessellation_mb = file_mb * 0.5;
    let total_mb = file_mb + entity_map_mb + tessellation_mb;

    let recommendation = if file_mb < 50.0 {
        "direct".to_string()
    } else if file_mb < 500.0 {
        "chunked".to_string()
    } else {
        "streaming".to_string()
    };

    ParseMemoryEstimate {
        file_content_mb: file_mb,
        entity_map_mb,
        tessellation_mb,
        total_mb,
        recommendation,
    }
}

/// Get file size without reading the content.
pub fn get_file_size(path: &str) -> Result<u64, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to get file metadata for '{}': {}", path, e))?;
    Ok(metadata.len())
}

/// Read a file efficiently for large files, using buffered reading.
/// For files under 50MB, reads into a String directly.
/// For files over 50MB, uses buffered reading with progress.
pub fn read_ifc_file_efficient(path: &str) -> Result<String, String> {
    let file_size = get_file_size(path)?;
    let threshold = 50 * 1024 * 1024; // 50 MB

    if file_size < threshold {
        // Small file: read directly
        std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))
    } else {
        // Large file: buffered reading with pre-allocated string
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open file '{}': {}", path, e))?;
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8MB buffer
        let mut content = String::with_capacity(file_size as usize);
        reader.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
        Ok(content)
    }
}

/// Read an IFC file in chunks, parsing each chunk's entities incrementally.
/// This avoids having the full file content + full parsed structure in memory simultaneously.
///
/// Opens the file with a `BufReader`, reads the HEADER section, then reads the DATA
/// section line by line, parsing each entity line using the existing `parse_entity_instance()`
/// and adding to the HashMap incrementally.
pub fn parse_ifc_file_chunked(path: &str) -> Result<IfcFile, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file '{}': {}", path, e))?;
    let reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8MB buffer

    let mut header = IfcHeader::default();
    let mut entities: HashMap<EntityId, IfcEntity> = HashMap::new();

    // State machine for tracking which section we're in
    #[derive(PartialEq)]
    enum Section {
        Preamble,
        Header,
        Data,
        Done,
    }

    let mut section = Section::Preamble;
    let mut multi_line_buffer = String::new();

    for line_result in reader.lines() {
        let line = line_result
            .map_err(|e| format!("Failed to read line: {}", e))?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        match section {
            Section::Preamble => {
                // Look for ISO header and HEADER; marker
                let upper = trimmed.to_uppercase();
                if upper.contains("HEADER;") || upper.starts_with("HEADER;") {
                    section = Section::Header;
                }
            }
            Section::Header => {
                // Scan header for schema info
                let upper = trimmed.to_uppercase();
                if upper.starts_with("FILE_SCHEMA") {
                    if let Some(start) = trimmed.find('\'') {
                        if let Some(end) = trimmed[start + 1..].find('\'') {
                            let _schema = &trimmed[start + 1..start + 1 + end];
                        }
                    }
                } else if upper.starts_with("FILE_NAME") {
                    if let Some(start) = trimmed.find('\'') {
                        if let Some(end) = trimmed[start + 1..].find('\'') {
                            header.file_name = trimmed[start + 1..start + 1 + end].to_string();
                        }
                    }
                }

                if upper.contains("ENDSEC;") {
                    section = Section::Data;
                    // Skip until we find "DATA;"
                }
            }
            Section::Data => {
                let upper = trimmed.to_uppercase();

                // Check for DATA; marker (might be on same line as ENDSEC; of header)
                if upper.starts_with("DATA;") {
                    continue;
                }

                // Check for end of data section
                if upper.starts_with("ENDSEC;") || upper.starts_with("END-ISO") {
                    // Flush any remaining multi-line buffer
                    if !multi_line_buffer.is_empty() {
                        let buf = std::mem::take(&mut multi_line_buffer);
                        let buf_trimmed = buf.trim();
                        if buf_trimmed.starts_with('#') {
                            if let Ok((_, entity)) = parse_entity_instance(buf_trimmed) {
                                entities.insert(entity.id, entity);
                            }
                        }
                    }
                    section = Section::Done;
                    continue;
                }

                // Handle entity lines - they might span multiple lines
                if trimmed.starts_with('#') {
                    // If we have a previous multi-line entity, parse it first
                    if !multi_line_buffer.is_empty() {
                        let buf = std::mem::take(&mut multi_line_buffer);
                        let buf_trimmed = buf.trim();
                        if buf_trimmed.starts_with('#') {
                            if let Ok((_, entity)) = parse_entity_instance(buf_trimmed) {
                                entities.insert(entity.id, entity);
                            }
                        }
                    }

                    // Check if the line ends with a semicolon (complete entity)
                    if trimmed.ends_with(';') {
                        // Normalize CR if present
                        let normalized = trimmed.replace('\r', "");
                        let clean = normalized.trim();
                        if let Ok((_, entity)) = parse_entity_instance(clean) {
                            entities.insert(entity.id, entity);
                        }
                    } else {
                        // Start of a multi-line entity
                        multi_line_buffer = trimmed.to_string();
                    }
                } else if !multi_line_buffer.is_empty() {
                    // Continuation of a multi-line entity
                    multi_line_buffer.push(' ');
                    multi_line_buffer.push_str(trimmed);

                    if trimmed.ends_with(';') {
                        let buf = std::mem::take(&mut multi_line_buffer);
                        let buf_trimmed = buf.trim();
                        if buf_trimmed.starts_with('#') {
                            if let Ok((_, entity)) = parse_entity_instance(buf_trimmed) {
                                entities.insert(entity.id, entity);
                            }
                        }
                    }
                }
            }
            Section::Done => break,
        }
    }

    Ok(IfcFile {
        header,
        entities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_schema_ifc2x3() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2]'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'','','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
ENDSEC;
END-ISO-10303-21;"#;
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Ifc2x3);
    }

    #[test]
    fn test_detect_schema_ifc4() {
        let content = "FILE_SCHEMA(('IFC4'));";
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Ifc4);
    }

    #[test]
    fn test_detect_schema_ifc4x3() {
        let content = "FILE_SCHEMA(('IFC4X3'));";
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Ifc4x3);
    }

    #[test]
    fn test_detect_schema_ifc4x3_add2() {
        let content = "FILE_SCHEMA(('IFC4X3_ADD2'));";
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Ifc4x3);
    }

    #[test]
    fn test_detect_schema_unknown() {
        let content = "FILE_SCHEMA(('IFC5'));";
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Unknown("IFC5".to_string()));
    }

    #[test]
    fn test_detect_schema_missing() {
        let content = "just some random text";
        assert_eq!(detect_ifc_schema(content), IfcSchemaVersion::Unknown("".to_string()));
    }

    #[test]
    fn test_normalize_entity_name_facility() {
        assert_eq!(normalize_entity_name("IFCFACILITY"), "IFCBUILDING");
    }

    #[test]
    fn test_normalize_entity_name_unchanged() {
        assert_eq!(normalize_entity_name("IFCWALL"), "IFCWALL");
        assert_eq!(normalize_entity_name("IFCSLAB"), "IFCSLAB");
        assert_eq!(normalize_entity_name("IFCBUILDINGELEMENTPROXY"), "IFCBUILDINGELEMENTPROXY");
    }

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

    #[test]
    fn test_extract_map_conversion_none() {
        let entities: HashMap<i32, IfcEntity> = HashMap::new();
        assert!(extract_map_conversion(&entities).is_none());
    }

    #[test]
    fn test_extract_map_conversion_basic() {
        let mut entities = HashMap::new();

        // IFCPROJECTEDCRS(Name, Description, GeodeticDatum, VerticalDatum, MapProjection, MapZone, MapUnit)
        entities.insert(
            100,
            IfcEntity {
                id: 100,
                entity_type: "IFCPROJECTEDCRS".to_string(),
                attributes: vec![
                    IfcValue::String("EPSG:25832".to_string()),
                    IfcValue::String("UTM Zone 32N".to_string()),
                    IfcValue::String("ETRS89".to_string()),
                    IfcValue::Null,
                    IfcValue::String("Transverse Mercator".to_string()),
                    IfcValue::String("32".to_string()),
                    IfcValue::Null,
                ],
            },
        );

        // IFCMAPCONVERSION(SourceCRS, TargetCRS, Eastings, Northings, OrthogonalHeight, XAxisAbscissa, XAxisOrdinate, Scale)
        entities.insert(
            200,
            IfcEntity {
                id: 200,
                entity_type: "IFCMAPCONVERSION".to_string(),
                attributes: vec![
                    IfcValue::EntityRef(50), // SourceCRS
                    IfcValue::EntityRef(100), // TargetCRS
                    IfcValue::Real(500000.0),
                    IfcValue::Real(5700000.0),
                    IfcValue::Real(100.0),
                    IfcValue::Real(0.9996),
                    IfcValue::Real(0.0),
                    IfcValue::Real(1.0),
                ],
            },
        );

        let mc = extract_map_conversion(&entities).expect("Should parse MapConversion");
        assert!((mc.eastings - 500000.0).abs() < 1e-6);
        assert!((mc.northings - 5700000.0).abs() < 1e-6);
        assert!((mc.orthogonal_height - 100.0).abs() < 1e-6);
        assert_eq!(mc.crs_name, "EPSG:25832");
        assert_eq!(mc.geodetic_datum, "ETRS89");
        assert_eq!(mc.map_zone, "32");
    }

    #[test]
    fn test_detect_ifc_units_millimetres() {
        let mut entities = HashMap::new();

        // IFCSIUNIT for length: millimetres
        entities.insert(
            10,
            IfcEntity {
                id: 10,
                entity_type: "IFCSIUNIT".to_string(),
                attributes: vec![
                    IfcValue::Null,                          // Dimensions
                    IfcValue::Enum("LENGTHUNIT".to_string()), // UnitType
                    IfcValue::Enum("MILLI".to_string()),      // Prefix
                    IfcValue::Enum("METRE".to_string()),      // Name
                ],
            },
        );

        // IFCUNITASSIGNMENT
        entities.insert(
            1,
            IfcEntity {
                id: 1,
                entity_type: "IFCUNITASSIGNMENT".to_string(),
                attributes: vec![IfcValue::List(vec![IfcValue::EntityRef(10)])],
            },
        );

        let factors = detect_ifc_units(&entities);
        assert!((factors.length_factor - 0.001).abs() < 1e-9);
        assert!((factors.area_factor - 0.000001).abs() < 1e-12);
        assert!((factors.volume_factor - 0.000000001).abs() < 1e-15);
        assert!(factors.description.contains("mm"));
    }

    #[test]
    fn test_detect_ifc_units_default() {
        let entities: HashMap<i32, IfcEntity> = HashMap::new();
        let factors = detect_ifc_units(&entities);
        assert!((factors.length_factor - 1.0).abs() < 1e-9);
    }

    // ====================================================================
    // Streaming, Parallel, and Partial Parsing Tests
    // ====================================================================

    /// Helper: build a minimal valid IFC file string for testing.
    fn sample_ifc_content() -> String {
        r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2]'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0001',#2,'Test Project',$,$,$,$,$,$);
#2=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,0);
#3=IFCPERSONANDORGANIZATION(#5,#6,$);
#4=IFCAPPLICATION(#6,'1.0','TestApp','TestApp');
#5=IFCPERSON($,'Doe','John',$,$,$,$,$);
#6=IFCORGANIZATION($,'TestOrg',$,$,$);
#10=IFCWALL('W001',#2,'Wall A',$,$,#100,#200,$);
#11=IFCWALL('W002',#2,'Wall B',$,$,#101,#201,$);
#12=IFCSLAB('S001',#2,'Slab A',$,$,#102,#202,$);
#13=IFCCOLUMN('C001',#2,'Column A',$,$,#103,#203,$);
#14=IFCBEAM('B001',#2,'Beam A',$,$,#104,#204,$);
#15=IFCSHAPEREPRESENTATION(#50,'Body','SweptSolid',(#300));
#16=IFCEXTRUDEDAREASOLID(#301,#302,#303,3.0);
ENDSEC;
END-ISO-10303-21;"#
            .to_string()
    }

    #[test]
    fn test_streaming_parse_matches_regular() {
        let content = sample_ifc_content();
        let regular = IfcFile::parse(&content).expect("Regular parse should succeed");
        let streaming =
            StreamingIfcParser::parse_streaming(&content).expect("Streaming parse should succeed");

        // Both should produce the same set of entity IDs
        assert_eq!(regular.entities.len(), streaming.entities.len());
        for (id, entity) in &regular.entities {
            let streaming_entity = streaming
                .entities
                .get(id)
                .expect(&format!("Streaming result missing entity #{}", id));
            assert_eq!(entity.entity_type, streaming_entity.entity_type);
            assert_eq!(entity.attributes.len(), streaming_entity.attributes.len());
        }
    }

    #[test]
    fn test_parallel_parse_matches_regular() {
        let content = sample_ifc_content();
        let regular = IfcFile::parse(&content).expect("Regular parse should succeed");
        let parallel = parse_parallel(&content).expect("Parallel parse should succeed");

        assert_eq!(regular.entities.len(), parallel.entities.len());
        for (id, entity) in &regular.entities {
            let par_entity = parallel
                .entities
                .get(id)
                .expect(&format!("Parallel result missing entity #{}", id));
            assert_eq!(entity.entity_type, par_entity.entity_type);
            assert_eq!(entity.attributes.len(), par_entity.attributes.len());
        }
    }

    #[test]
    fn test_scan_entity_types() {
        let content = sample_ifc_content();
        let types = StreamingIfcParser::scan_entity_types(&content);

        // We have 13 entities in the sample
        assert_eq!(types.len(), 13);

        // Check some specific types are present
        let type_map: HashMap<EntityId, &String> =
            types.iter().map(|(id, t)| (*id, t)).collect();
        assert_eq!(type_map.get(&10).map(|s| s.as_str()), Some("IFCWALL"));
        assert_eq!(type_map.get(&11).map(|s| s.as_str()), Some("IFCWALL"));
        assert_eq!(type_map.get(&12).map(|s| s.as_str()), Some("IFCSLAB"));
        assert_eq!(type_map.get(&13).map(|s| s.as_str()), Some("IFCCOLUMN"));
        assert_eq!(type_map.get(&1).map(|s| s.as_str()), Some("IFCPROJECT"));
    }

    #[test]
    fn test_parse_filtered_walls_only() {
        let content = sample_ifc_content();
        let result = parse_filtered(&content, &["IFCWALL"]).expect("Filtered parse should succeed");

        // Only IFCWALL entities should be in the result
        assert_eq!(result.entities.len(), 2);
        for entity in result.entities.values() {
            assert_eq!(entity.entity_type, "IFCWALL");
        }
        // Verify the specific wall IDs
        assert!(result.entities.contains_key(&10));
        assert!(result.entities.contains_key(&11));
    }

    #[test]
    fn test_parse_preview_limits() {
        let content = sample_ifc_content();
        // Limit to 1 entity per type
        let result = parse_preview(&content, 1).expect("Preview parse should succeed");

        // Count how many of each type we got
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for entity in result.entities.values() {
            *type_counts.entry(entity.entity_type.clone()).or_insert(0) += 1;
        }

        // There are 2 IFCWALLs in the content, but we limited to 1 per type
        assert_eq!(*type_counts.get("IFCWALL").unwrap_or(&0), 1);
        // Other types that appear once should still be present
        assert_eq!(*type_counts.get("IFCSLAB").unwrap_or(&0), 1);
        assert_eq!(*type_counts.get("IFCCOLUMN").unwrap_or(&0), 1);
        assert_eq!(*type_counts.get("IFCPROJECT").unwrap_or(&0), 1);
    }

    #[test]
    fn test_scan_ifc_summary() {
        let content = sample_ifc_content();
        let summary = scan_ifc_summary(&content);

        assert_eq!(summary.schema, "IFC4");
        assert_eq!(summary.total_entities, 13);
        assert_eq!(*summary.entity_type_counts.get("IFCWALL").unwrap_or(&0), 2);
        assert_eq!(*summary.entity_type_counts.get("IFCSLAB").unwrap_or(&0), 1);
        assert_eq!(
            *summary.entity_type_counts.get("IFCCOLUMN").unwrap_or(&0),
            1
        );

        // estimated_elements: IFCWALL(2) + IFCSLAB(1) + IFCCOLUMN(1) + IFCBEAM(1) = 5
        assert_eq!(summary.estimated_elements, 5);
        // has_geometry: IFCSHAPEREPRESENTATION and IFCEXTRUDEDAREASOLID are present
        assert!(summary.has_geometry);
    }

    #[test]
    fn test_empty_data_section() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Test'),'2;1');
FILE_NAME('empty.ifc','2024-01-01',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
ENDSEC;
END-ISO-10303-21;"#;

        // Streaming parse of empty DATA section
        let streaming =
            StreamingIfcParser::parse_streaming(content).expect("Should handle empty DATA");
        assert_eq!(streaming.entities.len(), 0);

        // Parallel parse of empty DATA section
        let parallel = parse_parallel(content).expect("Should handle empty DATA");
        assert_eq!(parallel.entities.len(), 0);

        // Filtered parse of empty DATA section
        let filtered =
            parse_filtered(content, &["IFCWALL"]).expect("Should handle empty DATA");
        assert_eq!(filtered.entities.len(), 0);

        // Summary of empty DATA section
        let summary = scan_ifc_summary(content);
        assert_eq!(summary.total_entities, 0);
        assert_eq!(summary.estimated_elements, 0);
        assert!(!summary.has_geometry);

        // scan_entity_types on empty DATA section
        let types = StreamingIfcParser::scan_entity_types(content);
        assert!(types.is_empty());
    }

    #[test]
    fn test_partial_with_dependencies() {
        // Test that filtered parse can still get entities that reference other entities.
        // The filter only returns entities of the requested types, but each entity
        // is self-contained with its attributes (including EntityRef values).
        let content = sample_ifc_content();

        // Filter for walls and the project entity
        let result = parse_filtered(&content, &["IFCWALL", "IFCPROJECT"])
            .expect("Filtered parse should succeed");

        // Should have 2 walls + 1 project = 3 entities
        assert_eq!(result.entities.len(), 3);

        // The wall entities should still have their EntityRef attributes intact
        let wall = result.entities.get(&10).expect("Wall #10 should exist");
        assert_eq!(wall.entity_type, "IFCWALL");
        // The wall references #2 (IFCOWNERHISTORY) in its attributes - the ref
        // value is preserved even though #2 is not in the filtered result
        assert_eq!(wall.attributes.len(), 8);
        // Attribute 1 should be EntityRef(2) referencing IFCOWNERHISTORY
        match &wall.attributes[1] {
            IfcValue::EntityRef(id) => assert_eq!(*id, 2),
            other => panic!("Expected EntityRef(2), got {:?}", other),
        }

        // The project entity should be present too
        let project = result.entities.get(&1).expect("Project #1 should exist");
        assert_eq!(project.entity_type, "IFCPROJECT");
    }

    // ====================================================================
    // Efficient File Reading and Chunked Parsing Tests
    // ====================================================================

    #[test]
    fn test_estimate_parse_memory_small_file() {
        // 10 MB file
        let estimate = estimate_parse_memory(10 * 1024 * 1024);
        assert!((estimate.file_content_mb - 10.0).abs() < 0.01);
        assert!((estimate.entity_map_mb - 15.0).abs() < 0.01);
        assert!((estimate.tessellation_mb - 5.0).abs() < 0.01);
        assert!((estimate.total_mb - 30.0).abs() < 0.01);
        assert_eq!(estimate.recommendation, "direct");
    }

    #[test]
    fn test_estimate_parse_memory_medium_file() {
        // 100 MB file
        let estimate = estimate_parse_memory(100 * 1024 * 1024);
        assert_eq!(estimate.recommendation, "chunked");
    }

    #[test]
    fn test_estimate_parse_memory_large_file() {
        // 1 GB file
        let estimate = estimate_parse_memory(1024 * 1024 * 1024);
        assert_eq!(estimate.recommendation, "streaming");
        assert!(estimate.total_mb > 1000.0);
    }

    #[test]
    fn test_get_file_size_nonexistent() {
        let result = get_file_size("/nonexistent/path/file.ifc");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_ifc_file_efficient_nonexistent() {
        let result = read_ifc_file_efficient("/nonexistent/path/file.ifc");
        assert!(result.is_err());
    }

    #[test]
    fn test_chunked_parse_from_tempfile() {
        // Write sample IFC content to a temp file and parse it with chunked parser
        let content = sample_ifc_content();
        let dir = std::env::temp_dir();
        let path = dir.join("test_chunked_parse.ifc");
        std::fs::write(&path, &content).expect("write temp file");

        let result = parse_ifc_file_chunked(path.to_str().unwrap());
        assert!(result.is_ok(), "Chunked parse should succeed: {:?}", result.err());
        let ifc_file = result.unwrap();

        // Should have parsed all 13 entities from the sample
        assert_eq!(ifc_file.entities.len(), 13);

        // Verify specific entities
        assert!(ifc_file.entities.contains_key(&1)); // IFCPROJECT
        assert!(ifc_file.entities.contains_key(&10)); // IFCWALL
        assert!(ifc_file.entities.contains_key(&12)); // IFCSLAB

        let wall = ifc_file.entities.get(&10).unwrap();
        assert_eq!(wall.entity_type, "IFCWALL");

        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_chunked_parse_nonexistent_file() {
        let result = parse_ifc_file_chunked("/nonexistent/path/file.ifc");
        assert!(result.is_err());
    }
}
