use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::Path};

/// Configuration for the generator
///
/// Allows you to read values from a file in JSON format:
/// <pre>
/// {
///  "include": {
///    "Order": "*",
///    "User": ["id", "email"]
///  },
///  "struct_derives": ["Clone", "Copy", "Debug", "Deserialize"],
/// }
/// </pre>
/// Or in YAML format:
/// <pre>
/// include:
///  Order: "*"
///  User:
///    - id
///    - email
/// struct_derives:
///    - Clone
///    - Copy
///    - Debug
///    - Deserialize
/// </pre>
#[derive(Deserialize)]
pub struct FilterConfig {
    /// Names of schemes to include in the generated file
    pub include: Option<HashMap<String, SchemaFilter>>,
    /// Names of schemes to exclude from the generated file
    pub exclude: Option<HashMap<String, SchemaFilter>>,
    /// Defines a list of `#[derive(...)]` when generating the structure. By
    /// default, `#[derive(Debug, Clone, serde::Deserialize)]`.
    #[serde(default = "get_default_struct_derives")]
    pub struct_derives: Vec<String>,
    /// Defines a list of `#[derive(...)]` when generating the enumeration. By
    /// default, `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd,
    /// Ord, serde::Deserialize)]`.
    #[serde(default = "get_default_enum_derives")]
    pub enum_derives: Vec<String>,
    /// automatically adds schemas to the filter if the fields of another
    /// schema refer to it
    #[serde(default)]
    pub auto_include_dependencies: bool,
}

impl std::default::Default for FilterConfig {
    fn default() -> Self {
        Self {
            struct_derives: get_default_struct_derives(),
            enum_derives: get_default_enum_derives(),
            include: Default::default(),
            exclude: Default::default(),
            auto_include_dependencies: Default::default(),
        }
    }
}

fn get_default_struct_derives() -> Vec<String> {
    vec!["Debug".into(), "Clone".into(), "Deserialize".into()]
}

fn get_default_enum_derives() -> Vec<String> {
    vec![
        "Debug".into(),
        "Clone".into(),
        "Copy".into(),
        "PartialEq".into(),
        "Eq".into(),
        "PartialOrd".into(),
        "Ord".into(),
        "Deserialize".into(),
    ]
}

/// Filter element: either "*" or an array of strings
#[derive(Deserialize)]
#[serde(untagged)]
pub enum SchemaFilter {
    /// All properties are selected, i.e. `*`
    #[allow(unused)]
    AcceptAll(String),
    /// Only some properties are selected
    AcceptSelected(Vec<String>),
}

impl SchemaFilter {
    pub(super) fn is_accepted(&self, name: &str) -> bool {
        match self {
            SchemaFilter::AcceptAll(_) => true,
            SchemaFilter::AcceptSelected(items) => items.iter().any(|i| i == name),
        }
    }
}

impl FilterConfig {
    /// Read configuration from string
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
            panic!("Couldn't determine the config file format");
        }
    }

    pub(super) fn is_schema_present(&self, schema_name: &str) -> bool {
        // Just check if the schema is found somewhere in the filter
        if let Some(schemas) = &self.include {
            return schemas.contains_key(schema_name);
        }
        if let Some(schemas) = &self.exclude {
            return schemas.contains_key(schema_name);
        }
        false
    }

    pub(super) fn is_schema_accepted(&self, schema_name: &str) -> bool {
        // if a list of inclusions is specified
        if let Some(schemas) = &self.include {
            // then only the listed ones are suitable
            return schemas.contains_key(schema_name);
        }
        // if a list of exceptions is specified
        if let Some(schemas) = &self.exclude {
            return match schemas.get(schema_name) {
                // it is suitable only if some fields are specified (we will
                // deal with them in method is_property_accepted)
                Some(schema) => matches!(schema, SchemaFilter::AcceptSelected(_)),
                // there is no such on the blacklist, it is suitable
                None => true,
            };
        }
        // if nothing is specified, then all are suitable
        true
    }

    pub(super) fn is_property_accepted(&self, schema_name: &str, property_name: &str) -> bool {
        if self.auto_include_dependencies && !self.is_schema_present(schema_name) {
            // in this case, automatic include is used
            return true;
        }
        // if a list of inclusions is specified, then only those listed are
        // suitable
        if let Some(schemas) = &self.include {
            return match schemas.get(schema_name) {
                Some(filter) => filter.is_accepted(property_name),
                None => false,
            };
        }
        // if a list of exceptions is specified, then only those listed are
        // not suitable
        if let Some(schemas) = &self.exclude {
            return match schemas.get(schema_name) {
                Some(filter) => !filter.is_accepted(property_name),
                None => true,
            };
        }
        // if nothing is specified, then all are suitable
        true
    }
}
