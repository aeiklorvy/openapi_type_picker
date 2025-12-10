use crate::*;

#[test]
fn test_array_alias() {
    const SCHEMA: &str = r##"
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
          format: int64
        name:
          type: string
        tag:
          type: string
      required:
        - id
        - name
        - tag
    Pets:
      type: array
      items:
        $ref: '#/components/schemas/Pet'
"##;

    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::default();
    let s = generate_openapi_types(openapi, config).unwrap();

    assert!(s.contains("pub type Pets = Vec<Pet>;"));
}
