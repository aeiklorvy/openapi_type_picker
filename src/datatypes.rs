/// Representation of a schema as a data type - a structure, enumeration, or
/// type alias
pub enum DataType {
    /// The most common structure
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    /// A flat enumeration, where each item is represented by a number
    /// (also called unit-only enum)
    Enum { name: String, items: Vec<String> },
    /// An extra name for existing type
    Alias {
        alias: String,
        // Use the StructField to represent the existing type information
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

    /// Sorts the fields in alphabetical order
    pub fn sort_fields(&mut self) {
        match self {
            DataType::Struct { fields, .. } => {
                fields.sort_by(|a, b| a.name.cmp(&b.name));
            }
            DataType::Enum { items, .. } => items.sort(),
            DataType::Alias { .. } => (),
        }
    }
}

/// Representation of schema object properties as structure fields
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field type
    pub type_: FieldType,
    /// "format" property, is present
    pub type_format: String,
    /// The dimension of array:
    /// * 0 = is not an array,
    /// * 1 = is a flat array,
    /// * 2 = an array of arrays (matrix)
    /// * and so on
    pub array_dimensions: i32,
    /// Can be null or not
    pub is_nullable: bool,
    /// Comments
    pub descr: String,
}

pub enum FieldType {
    /// Just type name
    Plain(String),
    /// Type name can be one of these values
    OneOf(Vec<String>),
}

impl FieldType {
    /// Returns a [`Vec`] of all possible field types
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            FieldType::Plain(t) => vec![t.clone()],
            FieldType::OneOf(items) => items.clone(),
        }
    }
}
