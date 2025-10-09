use crate::*;

#[test]
fn test_oneof_ref() {
    const SCHEMA: &str = r##"
components:
  schemas:
    First:
      type: object
      properties:
        name:
          type: string
    Second:
      type: object
      properties:
        value:
          type: number
    Mixed:
      type: object
      properties:
        firstOrSecond:
          oneOf:
            - $ref: '#/components/schemas/First'
            - $ref: '#/components/schemas/Second'
"##;

    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::default();
    let s = generate_openapi_types(openapi, config).unwrap();

    // check "Mixed" is generated right
    assert!(s.contains(
        r#"#[derive(Debug, Clone, Deserialize)]
pub struct Mixed {
    #[serde(rename = "firstOrSecond")]
    pub first_or_second: _UnionFirstOrSecond,
}"#
    ));

    // check helper types is also generated
    assert!(s.contains(
        r#"#[derive(Debug, Clone, Deserialize)]
pub enum _UnionFirstOrSecond {
    First(First),
    Second(Second),
}"#
    ));
}
