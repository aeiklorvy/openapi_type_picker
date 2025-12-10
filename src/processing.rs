use crate::datatypes::{DataType, FieldType, StructField};
use crate::filter::FilterConfig;
use crate::openapi::{OpenApi, Schema};
use std::error::Error;

/// Performs schema parsing from the OpenAPI specification
pub fn process_components(
    spec: &OpenApi,
    filter: &FilterConfig,
) -> Result<Vec<DataType>, Box<dyn Error>> {
    let mut dependencies = vec![];
    let mut datatypes = vec![];

    for (schema_name, definition) in spec.components.schemas.iter() {
        if !filter.is_schema_accepted(schema_name) {
            continue;
        }
        let datatype = process_schema(schema_name, definition, filter)?;
        datatypes.push(datatype);
        if filter.auto_include_dependencies {
            find_dependend_schemas(&schema_name, spec, filter, &mut dependencies);
        }
    }

    for schema_name in dependencies {
        if datatypes.iter().any(|dt| dt.schema_name() == schema_name) {
            // already added
            continue;
        }
        if let Some(definition) = spec.components.schemas.get(&schema_name) {
            datatypes.push(process_schema(&schema_name, definition, filter)?);
        }
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
            properties,
            enum_items,
            required,
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
                    let field = process_schema_property(
                        schema_name,
                        &prop_name,
                        prop_definition,
                        required.contains(prop_name),
                    )?;
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
            } else {
                // let's assume that this is a type alias
                Ok(DataType::Alias {
                    alias: schema_name.to_owned(),
                    info: process_schema_property(schema_name, "", definition, true)?,
                })
            }
        }
    }
}

/// Performs analysis of a single schema property
fn process_schema_property(
    schema_name: &str,
    name: &str,
    definition: &Schema,
    is_required: bool,
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
                type_: FieldType::Plain(
                    ref_.strip_prefix("#/components/schemas/")
                        .unwrap()
                        .to_owned(),
                ),
                type_format: String::new(),
                array_dimensions: 0,
                is_nullable: false,
                descr: String::new(),
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
            any_of,
            one_of,
            ..
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
                let mut field = process_schema_property(schema_name, name, items, is_required)?;
                field.array_dimensions += 1;
                // trying to account for nullable
                field.is_nullable = field.is_nullable | *nullable;
                // trying to fill the description
                if field.descr.is_empty() && !description.is_empty() {
                    field.descr = description.clone();
                }
                Ok(field)
            } else if let Some(schemas) = all_of {
                if schemas.len() > 1 {
                    // composition currently not supported
                    return Err(
                        format!("{schema_name:?}.{name:?}: expected one ref in `allOf`").into(),
                    );
                }
                // behaves like a simple ref in this case
                let mut field =
                    process_schema_property(schema_name, name, &schemas[0], is_required)?;
                // trying to account for nullable
                field.is_nullable = field.is_nullable | *nullable;
                // trying to fill the description
                if field.descr.is_empty() && !description.is_empty() {
                    field.descr = description.clone();
                }
                Ok(field)
            } else if let Some(schemas) = one_of {
                // field can have one of the specified types
                let mut types = vec![];
                for schema in schemas {
                    let field = process_schema_property(schema_name, name, schema, is_required)?;
                    types.extend(field.type_.to_vec());
                }
                Ok(StructField {
                    name: name.to_owned(),
                    type_: FieldType::OneOf(types),
                    type_format: String::new(),
                    array_dimensions: 0,
                    is_nullable: *nullable,
                    descr: description.clone(),
                })
            } else if let Some(_) = any_of {
                Err(format!("{schema_name:?}.{name:?}: `anyOf` is not supported").into())
            } else if !schema_type.is_empty() {
                // in this case, it's a primitive type
                return Ok(StructField {
                    name: name.to_owned(),
                    type_: FieldType::Plain(schema_type.clone()),
                    type_format: format.clone(),
                    is_nullable: *nullable | !is_required,
                    descr: description.clone(),
                    array_dimensions: 0,
                });
            } else {
                // nothing is specified, not even type - believe that the field can be any object
                Ok(StructField {
                    name: String::new(),
                    type_: FieldType::Plain("object".into()),
                    type_format: String::new(),
                    array_dimensions: 0,
                    is_nullable: *nullable | !is_required,
                    descr: description.clone(),
                })
            }
        }
    }
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
pub fn find_missing_schemas(datatypes: &[DataType]) -> Vec<String> {
    let mut missing_schemas = vec![];

    for dt in datatypes {
        match dt {
            DataType::Enum { .. } => (),
            DataType::Struct { fields, .. } => {
                for field in fields {
                    for t in field.type_.to_vec() {
                        // not looking for primitive types, they are always there
                        if !is_primitive_type(&t) {
                            // trying to find the field type in the datatypes list
                            if !datatypes.iter().any(|dt| dt.schema_name() == t) {
                                // didn't find it, it means an error
                                missing_schemas.push(t.clone());
                            }
                        }
                    }
                }
            }
            DataType::Alias { info, .. } => {
                // not looking for primitive types, they are always there
                for t in info.type_.to_vec() {
                    if !is_primitive_type(&t) {
                        // trying to find the field type in the datatypes list
                        if !datatypes.iter().any(|dt| dt.schema_name() == t) {
                            // didn't find it, it means an error
                            missing_schemas.push(t.clone());
                        }
                    }
                }
            }
        }
    }

    missing_schemas.dedup();
    missing_schemas
}

fn is_primitive_type(typename: &str) -> bool {
    matches!(
        typename,
        "string" | "number" | "boolean" | "integer" | "array" | "object"
    )
}

/// Populates a [`Vec`] of dependent schemas via recursion
fn find_dependend_schemas(
    schema_name: &str,
    spec: &OpenApi,
    filter: &FilterConfig,
    dependencies: &mut Vec<String>,
) {
    if let Some(definition) = spec.components.schemas.get(schema_name) {
        if let Ok(dt) = process_schema(schema_name, definition, filter) {
            match dt {
                DataType::Struct { fields, .. } => {
                    for field in fields {
                        for t in field.type_.to_vec() {
                            if is_primitive_type(&t) {
                                continue;
                            }
                            if !dependencies.contains(&t) {
                                dependencies.push(t.clone());
                                find_dependend_schemas(&t, spec, filter, dependencies);
                            }
                        }
                    }
                }
                DataType::Alias { info, .. } => {
                    for t in info.type_.to_vec() {
                        if is_primitive_type(&t) {
                            continue;
                        }
                        if !dependencies.contains(&t) {
                            dependencies.push(t.clone());
                            find_dependend_schemas(&t, spec, filter, dependencies);
                        }
                    }
                }
                // refs in enums are not possible
                DataType::Enum { .. } => (),
            }
        }
    }
}
