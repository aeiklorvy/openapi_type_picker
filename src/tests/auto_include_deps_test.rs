use crate::*;

const SCHEMA: &str = r##"
components:
  schemas:
    Big:
      type: object
      properties:
        mediums:
          type: array
          items:
            $ref: '#/components/schemas/Medium'
    Medium:
      type: object
      properties:
        smalls:
          type: array
          items:
            $ref: '#/components/schemas/Small'
    Small:
      type: object
      properties:
        someNumber:
          type: number
        someString:
          type: string
        someBool:
          type: boolean
"##;

#[test]
fn test_auto_include_deps() {
    let filter = r#"
include:
  Big: '*'
  Small:
    - someString
auto_include_dependencies: true
"#;
    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::from_str(filter);

    let structs = processing::process_components(&openapi, &config).unwrap();
    assert!(processing::find_missing_schemas(&structs).is_empty());

    // check "Big" struct
    let big = structs.iter().find(|dt| dt.schema_name() == "Big").unwrap();
    assert!(
        matches!(big, datatypes::DataType::Struct { fields, .. } if fields.iter().all(|f| f.name == "mediums"))
    );

    // the "Medium" struct should be generated automatically,
    // despite the presence in the filter.
    let medium = structs
        .iter()
        .find(|dt| dt.schema_name() == "Medium")
        .unwrap();
    assert!(
        matches!(medium, datatypes::DataType::Struct { fields, .. } if fields.iter().all(|f| f.name == "smalls"))
    );

    // and the "Small" should contain only one specific field.
    let small = structs
        .iter()
        .find(|dt| dt.schema_name() == "Small")
        .unwrap();
    assert!(
        matches!(small, datatypes::DataType::Struct { fields, .. } if fields.iter().all(|f| f.name == "someString"))
    );
}

#[test]
fn test_no_auto_include_deps() {
    let filter = r#"
include:
  Big: '*'
  Small:
    - someString
"#;
    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::from_str(filter);

    let structs = processing::process_components(&openapi, &config).unwrap();
    // struct "Medium" is not present and not included in filter, so it's missing
    assert!(
        processing::find_missing_schemas(&structs)
            .iter()
            .all(|x| x == "Medium")
    );

    // check "Big" struct
    let big = structs.iter().find(|dt| dt.schema_name() == "Big").unwrap();
    assert!(
        matches!(big, datatypes::DataType::Struct { fields, .. } if fields.iter().all(|f| f.name == "mediums"))
    );

    // and the "Small" should contain only one specific field.
    let small = structs
        .iter()
        .find(|dt| dt.schema_name() == "Small")
        .unwrap();
    assert!(
        matches!(small, datatypes::DataType::Struct { fields, .. } if fields.iter().all(|f| f.name == "someString"))
    );
}
