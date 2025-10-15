use crate::datatypes::{DataType, FieldType};
use convert_case::{Case, Casing};
use std::fmt::{Result, Write};

/// Writes a description of the generated module (file)
pub fn write_comment_header<W: Write>(w: &mut W) -> Result {
    writeln!(w, "//! # OpenApi Types")?;
    writeln!(w, "//! GENERATED AUTOMATICALLY, ALL THE CHANGES")?;
    writeln!(w, "//! YOU MAKE WILL BE REWRITTEN DURING")?;
    writeln!(w, "//! THE NEXT BUILD")?;
    writeln!(w)?; // add newline
    Ok(())
}

/// Writes the generated structures to the module (file)
pub fn write_rust_code<W: Write>(
    w: &mut W,
    datatypes: &[DataType],
    struct_derives: Option<&Vec<String>>,
    enum_derives: Option<&Vec<String>>,
) -> Result {
    let indent = "    "; // 4 * <space>

    // Necessary auxiliary types that were not present in the schema. These
    // types are "invisible" and are only needed to ensure the correctness of
    // the generated code.
    let mut helper_types = vec![];

    writeln!(w, "use serde::Deserialize;")?;
    writeln!(w)?; // add newline

    for dt in datatypes {
        match dt {
            DataType::Struct { name, fields } => {
                // generate helper types
                for field in fields {
                    match &field.type_ {
                        FieldType::Plain(_) => (),
                        FieldType::OneOf(items) => {
                            let name = generate_union_name(&items);
                            if !helper_types.contains(&name) {
                                helper_types.push(name);
                                write_union_type(w, &items, struct_derives)?;
                            }
                        }
                    }
                }

                writeln!(w, "/// {name}")?; // keep the original name
                if let Some(derives) = struct_derives {
                    writeln!(w, "#[derive({})]", derives.join(", "))?;
                } else {
                    writeln!(w, "#[derive(Debug, Clone, Deserialize)]")?;
                }
                writeln!(w, "pub struct {} {{", name.to_case(Case::Pascal))?;
                for field in fields {
                    let rust_name = fix_rust_keyword(field.name.to_case(Case::Snake));

                    let mut t = match &field.type_ {
                        FieldType::Plain(t) => get_rust_type(t, &field.type_format),
                        FieldType::OneOf(items) => generate_union_name(&items),
                    };
                    for _ in 0..field.array_dimensions {
                        t = format!("Vec<{t}>");
                    }
                    if field.is_nullable {
                        t = format!("Option<{t}>");
                    }

                    if !field.descr.is_empty() {
                        for line in field.descr.trim().lines() {
                            writeln!(w, "{indent}/// {}", line.trim())?;
                        }
                    }
                    // if the name of the property differs according to the
                    // naming rules of Rust
                    if field.name != rust_name {
                        writeln!(w, "{indent}#[serde(rename = {:?})]", field.name)?;
                    }
                    // Special instructions are required for [`OffsetDateTime`]
                    if t == "time::OffsetDateTime" {
                        writeln!(w, "{indent}#[serde(with = \"time::serde::iso8601\")]")?;
                    } else if t == "Option<time::OffsetDateTime>" {
                        writeln!(
                            w,
                            "{indent}#[serde(with = \"time::serde::iso8601::option\", default)]"
                        )?;
                    }
                    writeln!(w, "{indent}pub {rust_name}: {t},")?;
                }
                writeln!(w, "}}\n")?;
            }
            DataType::Enum { name, items } => {
                writeln!(w, "/// {name}")?; // keep the original name
                if let Some(derives) = enum_derives {
                    writeln!(w, "#[derive({})]", derives.join(", "))?;
                } else {
                    writeln!(
                        w,
                        "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]"
                    )?;
                }
                writeln!(w, "pub enum {} {{", name.to_case(Case::Pascal))?;
                for item in items {
                    let rust_name = item.to_case(Case::Pascal);
                    // if the name of the property differs according to the
                    // naming rules of Rust
                    if rust_name != *item {
                        writeln!(w, "{indent}#[serde(rename = {item:?})]")?;
                    }
                    writeln!(w, "{indent}{rust_name},")?;
                }
                writeln!(w, "}}\n")?;
                write_display_impl_for_enum(w, dt)?;
            }
            DataType::Alias { alias, info } => {
                let mut t = match &info.type_ {
                    FieldType::Plain(t) => get_rust_type(t, &info.type_format),
                    FieldType::OneOf(items) => {
                        let name = generate_union_name(&items);
                        if !helper_types.contains(&name) {
                            helper_types.push(name.clone());
                            write_union_type(w, &items, struct_derives)?;
                        }
                        name
                    }
                };

                for _ in 0..info.array_dimensions {
                    t = format!("Vec<{t}>");
                }
                if info.is_nullable {
                    t = format!("Option<{t}>");
                }

                writeln!(w, "/// {alias}")?; // keep the original name
                writeln!(w, "pub type {} = {t};\n", alias.to_case(Case::Pascal))?;
            }
        }
    }
    Ok(())
}

/// Writes a [`Display`](std::fmt::Display) implementation for enum
fn write_display_impl_for_enum<W: Write>(w: &mut W, dt: &DataType) -> Result {
    let indent1 = " ".repeat(4);
    let indent2 = " ".repeat(8);
    let indent3 = " ".repeat(12);
    if let DataType::Enum { name, items } = dt {
        let enum_name = name.to_case(Case::Pascal);
        writeln!(w, "impl std::fmt::Display for {enum_name} {{")?;
        writeln!(
            w,
            "{indent1}fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{"
        )?;
        writeln!(w, "{indent2}match self {{")?;
        for item in items {
            let item_name = item.to_case(Case::Pascal);
            // writes a non-distorted name
            writeln!(
                w,
                "{indent3}{enum_name}::{item_name} => write!(f, \"{item}\"),"
            )?;
        }
        writeln!(w, "{indent2}}}")?;
        writeln!(w, "{indent1}}}")?;
        writeln!(w, "}}\n")?;
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
            // the expected value is in RFC 3339/ISO 8601, "2017-07-21"
            "date" => "time::Date".to_owned(),
            // the expected value is in RFC 3339/ISO 8601, "2017-07-21T17:32:28Z"
            "date-time" => "time::OffsetDateTime".to_owned(),
            _ => "String".to_owned(),
        },
        "integer" => match format {
            "int32" => "i32".to_owned(),
            "int64" => "i64".to_owned(),
            _ => "i32".to_owned(),
        },
        "object" => "serde_json::Value".to_owned(),
        // otherwise, it is the name of a type (struct, enum or alias)
        struct_name => struct_name.to_case(Case::Pascal),
    }
}

/// Generates a name for the auxiliary structure, for example,
/// `UnionNumberOrString`.
fn generate_union_name(one_of: &[String]) -> String {
    format!("_Union{}", one_of.join("Or").to_case(Case::Pascal))
}

/// Writes an "invisible" auxiliary structure
fn write_union_type<W: Write>(
    w: &mut W,
    one_of: &[String],
    struct_derives: Option<&Vec<String>>,
) -> Result {
    let indent = "    "; // 4 * <space>

    // yes, this is an enum, but it is used only for combining structs, so
    // derives from structs are used
    if let Some(derives) = struct_derives {
        writeln!(w, "#[derive({})]", derives.join(", "))?;
    } else {
        writeln!(w, "#[derive(Debug, Clone, Deserialize)]")?;
    }
    writeln!(w, "#[serde(untagged)]")?;
    writeln!(w, "pub enum {} {{", generate_union_name(one_of))?;
    for t in one_of {
        let name = t.to_case(Case::Pascal);
        writeln!(w, "{indent}{name}({name}),")?;
    }
    writeln!(w, "}}\n")
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
