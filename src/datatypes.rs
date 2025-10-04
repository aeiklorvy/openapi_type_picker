/// Representation of a schema as a data type - a structure, enumeration, or
/// type alias
#[derive(Debug)]
pub enum DataType {
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
    pub fn schema_name(&self) -> &str {
        match self {
            DataType::Struct { name, .. } => &name,
            DataType::Enum { name, .. } => &name,
            DataType::Alias { alias, .. } => &alias,
        }
    }
}

/// Representation of schema object properties as structure fields
#[derive(Debug, Default)]
pub struct StructField {
    pub name: String,
    pub type_: String,
    pub type_format: String,
    pub array_dimensions: i32,
    pub is_nullable: bool,
    pub descr: String,
}
