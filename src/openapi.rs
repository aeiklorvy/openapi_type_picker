use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// OpenAPI root (minimal)
///
/// For a reference on the structure, see
/// <https://learn.openapis.org/specification/>
#[derive(Deserialize)]
pub struct OpenApi {
    pub components: Components,
}

#[derive(Deserialize)]
pub struct Components {
    pub schemas: HashMap<String, Schema>,
}

/// Universal description of the scheme
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Schema {
    /// Link to another scheme
    Ref {
        #[serde(rename = "$ref")]
        ref_: String,
    },
    /// Scheme with "type" property
    Typed {
        #[serde(rename = "type", default)]
        schema_type: String,

        #[serde(default)]
        format: String,

        #[serde(default)]
        nullable: bool,

        #[serde(default)]
        description: String,

        /// Object properties if `{"type": "object"}`
        properties: Option<HashMap<String, Schema>>,

        #[serde(default)]
        required: Vec<String>,

        /// Array items if `{"type": "array"}`
        items: Option<Box<Schema>>,

        /// Enumeration elements
        #[serde(rename = "enum")]
        enum_items: Option<Vec<String>>,

        /// Compositions
        #[serde(rename = "allOf")]
        all_of: Option<Vec<Schema>>,
        #[serde(rename = "oneOf")]
        one_of: Option<Vec<Schema>>,
        #[serde(rename = "anyOf")]
        any_of: Option<Vec<Schema>>,
    },
}

impl OpenApi {
    /// Read specifiaction from string
    pub fn from_str(data: &str) -> Self {
        if data.trim_start().starts_with("{") {
            serde_json::from_str(data).unwrap()
        } else {
            serde_yaml::from_str(data).unwrap()
        }
    }

    /// Read specifiaction from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let path_ref = path.as_ref();
        if path_ref.extension().unwrap() == "json" {
            serde_json::from_reader(File::open(path_ref).unwrap()).unwrap()
        } else if path_ref.extension().unwrap() == "yaml" {
            serde_yaml::from_reader(File::open(path_ref).unwrap()).unwrap()
        } else {
            panic!("Couldn't determine the schema file format");
        }
    }
}
