use crate::*;

#[test]
fn test_optional_property_ref() {
    const SCHEMA: &str = r##"
components:
  schemas:
    LogLevel:
      description: List differents levels of logs
      type: string
      enum:
        - UNKNOWN
        - TRACE
        - WARNING
        - INFO
    Alert:
      description: An alert
      type: object
      required:
        - code
      properties:
        code:
          type: string
          description: The error code
        log_level:
          description: Severity of the alert.
          $ref: '#/components/schemas/LogLevel'
        url:
          type: string
          description: Origin
"##;

    let openapi = OpenApi::from_str(SCHEMA);
    let config = FilterConfig::default();

    let openapi_types = generate_openapi_types(openapi, config).unwrap();

    // Make sure properties not required are wrapped in `Option`
    assert!(openapi_types.contains("log_level: Option<LogLevel>"));
    assert!(openapi_types.contains("url: Option<String>"));
}
