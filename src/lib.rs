#![doc = include_str!("../README.md")]

mod filter;
mod openapi;

use convert_case::{Case, Casing};
pub use filter::FilterConfig;
pub use openapi::OpenApi;
use openapi::Schema;

use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Representation of a schema as a data type - a structure, enumeration, or
/// type alias
#[derive(Debug)]
enum DataType {
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        items: Vec<String>,
    },
    Alias {
        alias: String,
        // Use the StructField to represent the type information
        info: StructField,
    },
}

impl DataType {
    /// The name of the schema that the data type represents
    fn scheme_name(&self) -> &str {
        match self {
            DataType::Struct { name, .. } => &name,
            DataType::Enum { name, .. } => &name,
            DataType::Alias { alias, .. } => &alias,
        }
    }
}

/// Representation of schema object properties as structure fields
#[derive(Debug, Default)]
struct StructField {
    name: String,
    type_: String,
    type_format: String,
    array_dimensions: i32,
    is_nullable: bool,
    descr: String,
}

/// Generates types according to the OpenAPI specification in a file. If it
/// fails, it returns an error, and the specified file will be cleared (if it
/// exists and it was opened).
///
/// # Example
/// ```
/// generate_openapi_types(
///     OpenApi::from_file("../schema.json"),
///     FilterConfig::from_file("../config.json")),
///     "src/api/types.rs"
/// ).unwrap();
/// ```
pub fn generate_openapi_types<P: AsRef<Path>>(
    openapi: OpenApi,
    config: FilterConfig,
    out_file: P,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_file)?;

    let structs = process_components(&openapi, &config)?;
    let missing_schemas = find_missing_schemas(&structs);
    if !missing_schemas.is_empty() {
        let msg = format!("Found reference to missing schemas: {:?}", missing_schemas);
        return Err(msg.into());
    }

    write_comment_header(&mut file)?;
    write_rust_code(
        &mut file,
        &structs,
        config.struct_derives.as_ref(),
        config.enum_derives.as_ref(),
    )?;
    Ok(())
}

/// Creates a document of the generated module (file)
fn write_comment_header(file: &mut File) -> std::io::Result<()> {
    writeln!(file, "//! # OpenApi Types")?;
    writeln!(file, "//! GENERATED AUTOMATICALLY, ALL THE CHANGES")?;
    writeln!(file, "//! YOU MAKE WILL BE REWRITTEN DURING")?;
    writeln!(file, "//! THE NEXT BUILD")?;
    writeln!(file)?; // add newline
    Ok(())
}

/// Writes the generated structures to the module (file)
fn write_rust_code(
    file: &mut File,
    datatypes: &[DataType],
    struct_derives: Option<&Vec<String>>,
    enum_derives: Option<&Vec<String>>,
) -> std::io::Result<()> {
    writeln!(file, "use serde::Deserialize;")?;
    writeln!(file)?;
    let indent = "    "; // 4 * <space>
    for dt in datatypes {
        match dt {
            DataType::Struct { name, fields } => {
                writeln!(file, "/// {name}")?; // keep the original name
                if let Some(derives) = struct_derives {
                    writeln!(file, "#[derive({})]", derives.join(", "))?;
                } else {
                    writeln!(file, "#[derive(Debug, Clone, Deserialize)]")?;
                }
                writeln!(file, "pub struct {} {{", name.to_case(Case::Pascal))?;
                for field in fields {
                    let rust_name = fix_rust_keyword(field.name.to_case(Case::Snake));

                    let mut t = get_rust_type(&field.type_, &field.type_format);
                    for _ in 0..field.array_dimensions {
                        t = format!("Vec<{t}>");
                    }
                    if field.is_nullable {
                        t = format!("Option<{t}>");
                    }

                    if !field.descr.is_empty() {
                        for line in field.descr.trim().lines() {
                            writeln!(file, "{indent}/// {}", line.trim())?;
                        }
                    }
                    // if the name of the property differs according to the
                    // naming rules of Rust
                    if field.name != rust_name {
                        writeln!(file, "{indent}#[serde(rename = {:?})]", field.name)?;
                    }
                    writeln!(file, "{indent}pub {rust_name}: {t},")?;
                }
                writeln!(file, "}}\n")?;
            }
            DataType::Enum { name, items } => {
                writeln!(file, "/// {name}")?; // keep the original name
                if let Some(derives) = enum_derives {
                    writeln!(file, "#[derive({})]", derives.join(", "))?;
                } else {
                    writeln!(
                        file,
                        "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]"
                    )?;
                }
                writeln!(file, "pub enum {} {{", name.to_case(Case::Pascal))?;
                for item in items {
                    let rust_name = item.to_case(Case::Pascal);
                    // if the name of the property differs according to the
                    // naming rules of Rust
                    if rust_name != *item {
                        writeln!(file, "{indent}#[serde(rename = {item:?})]")?;
                    }
                    writeln!(file, "{indent}{rust_name},")?;
                }
                writeln!(file, "}}\n")?;
            }
            DataType::Alias { alias, info } => {
                writeln!(file, "/// {}", alias)?; // keep the original name
                let mut t = get_rust_type(&info.type_, &info.type_format);
                for _ in 0..info.array_dimensions {
                    t = format!("Vec<{t}>");
                }
                if info.is_nullable {
                    t = format!("Option<{t}>");
                }
                writeln!(file, "pub type {} = {t};\n", alias.to_case(Case::Pascal))?;
            }
        }
    }
    Ok(())
}

/// Performs schema parsing from the OpenAPI specification
fn process_components(
    spec: &OpenApi,
    filter: &FilterConfig,
) -> Result<Vec<DataType>, Box<dyn Error>> {
    let mut datatypes = vec![];
    for (schema_name, definition) in spec.components.schemas.iter() {
        if !filter.is_schema_accepted(schema_name) {
            continue;
        }
        let datatype = process_schema(schema_name, definition, filter)?;
        datatypes.push(datatype);
    }
    Ok(datatypes)
}

/// Performs parsing of a single schema from the OpenAPI specification
fn process_schema(
    schema_name: &str,
    definition: &Schema,
    filter: &FilterConfig,
) -> Result<DataType, Box<dyn Error>> {
    match definition {
        Schema::Ref { .. } => {
            // An entry like `{"Schema": {"$ref": "#/..."}}` although it looks
            // like the truth, it doesn't make sense - why not write the
            // necessary structure right away? In general, I do not believe
            // that we will be able to meet this.
            let msg = format!("unexpected reference in root of {schema_name:?} definition");
            Err(msg.into())
        }
        Schema::Typed {
            schema_type,
            properties,
            enum_items,
            ..
        } => {
            // if the root element is an object, then it must have properties
            if let Some(props) = properties {
                // the object turns into a structure
                let mut fields = vec![];
                for (prop_name, prop_definition) in props {
                    if !filter.is_property_accepted(schema_name, prop_name) {
                        continue;
                    }
                    let field = process_schema_property(schema_name, &prop_name, prop_definition)?;
                    fields.push(field);
                }
                Ok(DataType::Struct {
                    name: schema_name.to_owned(),
                    fields,
                })
            } else if let Some(items) = enum_items {
                // this is an enum listing the options
                Ok(DataType::Enum {
                    name: schema_name.to_owned(),
                    items: items.clone(),
                })
            } else if is_primitive_type(&schema_type) {
                // is this a schema without "properties", but with a "type" -
                // is it legal? It's probably something like type alias
                Ok(DataType::Alias {
                    alias: schema_name.to_owned(),
                    info: process_schema_property(schema_name, "", definition)?,
                })
            } else {
                let msg = format!("root item of {schema_name:?} must be object");
                Err(msg.into())
            }
        }
    }
}

/// Performs analysis of a single schema property
fn process_schema_property(
    schema_name: &str,
    name: &str,
    definition: &Schema,
) -> Result<StructField, Box<dyn Error>> {
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        // or should I try to transform it?
        let msg = format!("property {schema_name:?}.{name:?} has untranslatable name");
        return Err(msg.into());
    }

    match definition {
        Schema::Ref { ref_ } => {
            return Ok(StructField {
                name: name.to_owned(),
                type_: ref_
                    .strip_prefix("#/components/schemas/")
                    .unwrap()
                    .to_owned(),
                ..Default::default()
            });
        }
        Schema::Typed {
            schema_type,
            format,
            nullable,
            description,
            properties,
            items,
            enum_items: _,
            all_of,
        } => {
            if let Some(_) = properties {
                // "properties" is specified, which means it is an object. We
                //  don't know how to generate nested objects.
                let msg = format!(
                    "property {schema_name:?}.{name:?} cannot be a nested object, use $ref instead"
                );
                Err(msg.into())
            } else if let Some(items) = items {
                // "items" is specified, this is an array
                let mut field = process_schema_property(schema_name, name, items)?;
                field.array_dimensions += 1;
                // trying to account for nullable
                field.is_nullable = field.is_nullable | *nullable;
                // trying to fill the description
                if field.descr.is_empty() && !description.is_empty() {
                    field.descr = description.clone();
                }
                Ok(field)
            } else if !schema_type.is_empty() {
                // in this case, it's a primitive type
                return Ok(StructField {
                    name: name.to_owned(),
                    type_: schema_type.clone(),
                    type_format: format.clone(),
                    is_nullable: *nullable,
                    descr: description.clone(),
                    ..Default::default()
                });
            } else {
                // nothing is specified, not even type - it's definitely a link
                if !all_of.is_empty() {
                    let mut field = process_schema_property(schema_name, name, &all_of[0])?;
                    // trying to account for nullable
                    field.is_nullable = field.is_nullable | *nullable;
                    // trying to fill the description
                    if field.descr.is_empty() && !description.is_empty() {
                        field.descr = description.clone();
                    }
                    Ok(field)
                } else {
                    let msg = format!(
                        "property {schema_name:?}.{name:?} is reference, but `allOf` is empty list"
                    );
                    Err(msg.into())
                }
            }
        }
    }
}

/// Returns the corresponding type for Rust
fn get_rust_type(typename: &str, format: &str) -> String {
    match typename {
        "number" => match format {
            "float" => "f32".to_owned(),
            "double" => "f64".to_owned(),
            _ => "f64".to_owned(),
        },
        "boolean" => "bool".to_owned(),
        "string" => match format {
            // the expected value is in RFC 3339, "2017-07-21"
            "date" => "time::Date".to_owned(),
            // the expected value is in RFC 3339, "2017-07-21T17:32:28Z"
            "date-time" => "time::OffsetDateTime".to_owned(),
            _ => "String".to_owned(),
        },
        "integer" => match format {
            "int32" => "i32".to_owned(),
            "int64" => "i64".to_owned(),
            _ => "i32".to_owned(),
        },
        // otherwise, it is the name of a type (struct, enum or alias)
        struct_name => struct_name.to_case(Case::Pascal),
    }
}

fn is_primitive_type(typename: &str) -> bool {
    matches!(
        typename,
        "string" | "number" | "boolean" | "integer" | "array"
    )
}

/// If the `name` matches the Rust keyword, a lower dash will be added to the
/// end of the `name`
fn fix_rust_keyword(name: String) -> String {
    if matches!(
        name.as_str(),
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    ) {
        return name + "_";
    }
    name
}

/// Using a filter, not all the necessary structures can be generated, which
/// is what we are trying to understand in order to avoid compilation
/// errors
///
/// You can explicitly include the generation of a schema property in the
/// filter, which will refer to other schema. If you also forget to include
/// the other schema that was referenced, the result will be a structure that
/// has a field with an undeclared type. Later, this will cause a compilation
/// error, so to simplify debugging, we immediately intercept such
/// situations.
fn find_missing_schemas(datatypes: &[DataType]) -> Vec<String> {
    let mut missing_schemas = vec![];

    for dt in datatypes {
        match dt {
            DataType::Enum { .. } => (),
            DataType::Struct { fields, .. } => {
                for field in fields {
                    // not looking for primitive types, they are always there
                    if !is_primitive_type(&field.type_) {
                        // trying to find the field type in the datatypes list
                        if !datatypes.iter().any(|dt| dt.scheme_name() == field.type_) {
                            // didn't find it, it means an error
                            missing_schemas.push(field.type_.clone());
                        }
                    }
                }
            }
            DataType::Alias { info, .. } => {
                // not looking for primitive types, they are always there
                if !is_primitive_type(&info.type_) {
                    // trying to find the field type in the datatypes list
                    if !datatypes.iter().any(|dt| dt.scheme_name() == info.type_) {
                        // didn't find it, it means an error
                        missing_schemas.push(info.type_.clone());
                    }
                }
            }
        }
    }

    missing_schemas.dedup();
    missing_schemas
}
