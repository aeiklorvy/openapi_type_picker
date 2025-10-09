#![doc = include_str!("../README.md")]

mod datatypes;
mod filter;
mod openapi;
mod processing;
mod writing;

#[cfg(test)]
mod tests;

// exported
pub use filter::FilterConfig;
pub use openapi::OpenApi;

use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generates types according to the OpenAPI specification to a file. If it
/// fails, it returns an error, and the specified file will be cleared (if it
/// exists and it was opened).
///
/// # Example
/// ```no_run
/// write_openapi_types(
///     OpenApi::from_file("../schema.json"),
///     FilterConfig::from_file("../config.json"),
///     "src/api/types.rs"
/// ).unwrap();
/// ```
pub fn write_openapi_types<P: AsRef<Path>>(
    openapi: OpenApi,
    config: FilterConfig,
    out_file: P,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_file)?;
    let s = generate_openapi_types(openapi, config)?;
    file.write_all(s.as_bytes())?;
    Ok(())
}

/// Generates types according to the OpenAPI specification to a [`String`]
///
/// # Example
/// ```no_run
/// let s = generate_openapi_types(
///     OpenApi::from_file("../schema.json"),
///     FilterConfig::from_file("../config.json")
/// ).unwrap();
/// println!("{s:?}");
/// ```
pub fn generate_openapi_types(
    openapi: OpenApi,
    config: FilterConfig,
) -> Result<String, Box<dyn Error>> {
    let mut datatypes = processing::process_components(&openapi, &config)?;
    let missing_schemas = processing::find_missing_schemas(&datatypes);
    if !missing_schemas.is_empty() {
        let msg = format!("Found reference to missing schemas: {:?}", missing_schemas);
        return Err(msg.into());
    }

    // sort data types to reduce the changes in the version control system
    datatypes.sort_by(|a, b| a.schema_name().cmp(b.schema_name()));
    for dt in &mut datatypes {
        dt.sort_fields();
    }

    let mut buf = String::with_capacity(1024);
    writing::write_comment_header(&mut buf)?;
    writing::write_rust_code(
        &mut buf,
        &datatypes,
        config.struct_derives.as_ref(),
        config.enum_derives.as_ref(),
    )?;

    Ok(buf)
}
