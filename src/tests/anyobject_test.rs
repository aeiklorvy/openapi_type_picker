use crate::*;

#[test]
fn test_any_object_1() {
    const SCHEMA: &str = r##"
components:
  schemas:
    AnyObject:
      description: "can be anything"
    NormalObject:
      type: object
      properties:
        value:
          type: number
        anyValue:
          $ref: '#/components/schemas/AnyObject'
"##;

    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::default();
    let types = processing::process_components(&openapi, &config).unwrap();

    for dt in types {
        match dt.schema_name() {
            "AnyObject" => {
                if let datatypes::DataType::Alias { info, .. } = dt {
                    assert!(info.type_.to_vec() == vec!["object"]);
                } else {
                    panic!("AnyObject is not type alias");
                }
            }
            "NormalObject" => {
                if let datatypes::DataType::Struct { fields, .. } = dt {
                    for field in fields {
                        if field.name == "value" {
                            assert_eq!(field.type_.to_vec(), vec!["number"]);
                        } else if field.name == "anyValue" {
                            assert_eq!(field.type_.to_vec(), vec!["AnyObject"]);
                        } else {
                            panic!("unexpected field {}", field.name);
                        }
                    }
                } else {
                    panic!("NormalObject is not struct");
                }
            }
            name => panic!("unknown schema name {name:?}"),
        }
    }
}

#[test]
fn test_any_object_2() {
    const SCHEMA: &str = r##"
components:
  schemas:
    NormalObject:
      type: object
      properties:
        value:
          type: number
        anyValue:
          type: object
"##;

    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::default();
    let types = processing::process_components(&openapi, &config).unwrap();

    for dt in types {
        match dt.schema_name() {
            "NormalObject" => {
                if let datatypes::DataType::Struct { fields, .. } = dt {
                    for field in fields {
                        if field.name == "value" {
                            assert_eq!(field.type_.to_vec(), vec!["number"]);
                        } else if field.name == "anyValue" {
                            assert_eq!(field.type_.to_vec(), vec!["object"]);
                        } else {
                            panic!("unexpected field {}", field.name);
                        }
                    }
                } else {
                    panic!("NormalObject is not struct");
                }
            }
            name => panic!("unknown schema name {name:?}"),
        }
    }
}
