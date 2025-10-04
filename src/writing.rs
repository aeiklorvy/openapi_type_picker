use crate::datatypes::DataType;
use convert_case::{Case, Casing};
use std::fs::File;
use std::io::{Result, Write};

/// Writes a description of the generated module (file)
pub fn write_comment_header(file: &mut File) -> Result<()> {
    writeln!(file, "//! # OpenApi Types")?;
    writeln!(file, "//! GENERATED AUTOMATICALLY, ALL THE CHANGES")?;
    writeln!(file, "//! YOU MAKE WILL BE REWRITTEN DURING")?;
    writeln!(file, "//! THE NEXT BUILD")?;
    writeln!(file)?; // add newline
    Ok(())
}

/// Writes the generated structures to the module (file)
pub fn write_rust_code(
    file: &mut File,
    datatypes: &[DataType],
    struct_derives: Option<&Vec<String>>,
    enum_derives: Option<&Vec<String>>,
) -> Result<()> {
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
                write_display_impl_for_enum(file, dt)?;
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

/// Writes a [`Display`](std::fmt::Display) implementation for enum
fn write_display_impl_for_enum(file: &mut File, dt: &DataType) -> Result<()> {
    let indent1 = " ".repeat(4);
    let indent2 = " ".repeat(8);
    let indent3 = " ".repeat(12);
    if let DataType::Enum { name, items } = dt {
        let enum_name = name.to_case(Case::Pascal);
        writeln!(file, "impl std::fmt::Display for {enum_name} {{")?;
        writeln!(
            file,
            "{indent1}fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{"
        )?;
        writeln!(file, "{indent2}match self {{")?;
        for item in items {
            let item_name = item.to_case(Case::Pascal);
            // writes a non-distorted name
            writeln!(
                file,
                "{indent3}{enum_name}::{item_name} => write!(f, \"{item}\"),"
            )?;
        }
        writeln!(file, "{indent2}}}")?;
        writeln!(file, "{indent1}}}")?;
        writeln!(file, "}}\n")?;
    }
    Ok(())
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
