#![doc = include_str!("../README.md")]

mod datatypes;
mod filter;
mod openapi;
mod processing;
mod writing;

// exported
pub use filter::FilterConfig;
pub use openapi::OpenApi;

use std::error::Error;
use std::fs::File;
use std::path::Path;

/// Generates types according to the OpenAPI specification in a file. If it
/// fails, it returns an error, and the specified file will be cleared (if it
/// exists and it was opened).
///
/// # Example
/// ```no_run
/// generate_openapi_types(
///     OpenApi::from_file("../schema.json"),
///     FilterConfig::from_file("../config.json"),
///     "src/api/types.rs"
/// ).unwrap();
/// ```
pub fn generate_openapi_types<P: AsRef<Path>>(
    openapi: OpenApi,
    config: FilterConfig,
    out_file: P,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_file)?;

    let structs = processing::process_components(&openapi, &config)?;
    let missing_schemas = processing::find_missing_schemas(&structs);
    if !missing_schemas.is_empty() {
        let msg = format!("Found reference to missing schemas: {:?}", missing_schemas);
        return Err(msg.into());
    }

    writing::write_comment_header(&mut file)?;
    writing::write_rust_code(
        &mut file,
        &structs,
        config.struct_derives.as_ref(),
        config.enum_derives.as_ref(),
    )?;
    Ok(())
}
