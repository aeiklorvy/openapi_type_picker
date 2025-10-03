## Quick start

Let's assume that I have this description of the shema (https://learn.openapis.org/examples/v3.0/petstore.html):
```json
{
  "openapi": "3.0.0",
  "info": {
    "version": "1.0.0",
    "title": "Swagger Petstore",
    "license": {
      "name": "MIT"
    }
  },
  "paths": { ... },
  "components": {
    "schemas": {
      "Pet": {
        "type": "object",
        "required": ["id", "name"],
        "properties": {
          "id": {
            "type": "integer",
            "format": "int64"
          },
          "name": {
            "type": "string"
          },
          "tag": {
            "type": "string"
          }
        }
      },
      "Pets": {
        "type": "array",
        "maxItems": 100,
        "items": {
          "$ref": "#/components/schemas/Pet"
        }
      },
    }
  }
}
```
And there is also such a configuration file, because I don't need all the fields of all data types:
```json
{
  "include": {     // generate only "Pet" struct
    "Pet": ["id"], // only with "id" property
  }
}
```
In order to generate a file with the definition of data types, in the file `build.rs` we need to add a function call:
```rust
use openapi_type_picker::*;
fn main() {
    generate_openapi_types(
        OpenApi::from_file("../schema.json"),
        FilterConfig::from_file("../config.json"),
        "src/api/types.rs" // path to output file
    ).unwrap();
}
```
After executing the `cargo build` command, following the path specified in the function, you can find the generated file with contents:
```rust
// src/api/types.rs
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct Pet {
    pub id: i64,
}
```

## How filters work

Filters allow you to include or exclude certain schemes from the generated file. For example, if the API describes several hundred methods, but only a small part is used for your project.

It is not necessary to add filters. By default (`FilterConfig::default()` or json `{}`), all types with all fields will be generated.

JSON and YAML formats with automatic detection are supported, but all the examples given will be in JSON format.

The configuration file allows you to configure the following:
- `include`: schemas to be generated;
- `exclude`: schemas that do not need to be generated;
- `struct_derives`: defines a list of `#[derive(...)]` when generating the structure, by default `["Debug", "Clone", "Deserialize"]`;
- `enum_derives`: defines a list of `#[derive(...)]` when generating an enumeration, by default `["Debug", "Clone", "Copy", "PartialEq", "Eq", "PartialOrd", "Ord", "Deserialize"]`.

For examples, the petstore demo scheme is used from the very beginning: https://learn.openapis.org/examples/v3.0/petstore.html.

The values `include` and `exclude` must be an object from the schema name and its fields. For example, such a configuration means that you only need to generate a structure for the `Pet` schema, which should have only the `id` field:
```json
{
  "include": {
    "Pet": ["id"],
  }
}
```
But this configuration means that the `Pet` structure with all its fields must be generated:
```json
{
  "include": {
    "Pet": "*",
  }
}
```
The exact same format is used to describe exclusions, i.e. structures that do not need to be generated. For example, this config says to generate all structures except `Pet`:
```json
{
  "exclude": {
    "Pet": "*",
  }
}
```
But if the schema has only a specific list of properties, then the corresponding structure will still be generated, but without the specified fields (in this example, only `Pet::id` will be generated):
```json
{
  "exclude": {
    "Pet": ["name", "tag"],
  }
}
```
If you want to define the custom behavior of the generated data types, then use `struct_derives` for structures and `enum_derives` for enumerations:
```json
{
  "include": {
    "Pet": ["id"],
  },
  "struct_derives": ["Clone", "Copy", "PartialEq", "Eq", "PartialOrd", "Ord"]
}
```
```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pet {
    pub id: i64,
}
```

**Important note on the operation of filters:**
> If a schema is included in the config, the property of which represents a link to another schema, then this "child" schema should also be included in the config. Otherwise, the generator will give an error about the presence of a field with a type link, the structure for which will not be generated.

At first, I thought of solving this through a recursive dependency lookup. But then, if the found schemes had not been included in the filter, their generation would have caused a contradiction. If these schemes had been included in the filter, they would still have been generated. Basically, it's working the way it is right now.

## Automatic generation during build

In order for the data types to be generated automatically during the project build, add an build dependency to `Cargo.toml`:
```toml
[build-dependencies]
openapi_type_picker = "0.1"
```
And then in the `build.rs` it will be possible to configure type generation:
```rust
use openapi_type_picker::*;
fn main() {
    // loading the openapi specification
    let openapi = OpenApi::from_file("../schema.json");
    // also can load from string:
    // let openapi = OpenApi::from_str(include_str!("../schema.json"));

    // loading the config file
    let config = FilterConfig::from_file("../config.json");
    // also can load from string:
    // let config = FilterConfig::from_str(include_str!("../config.json")));
    // or set manually:
    // let config = FilterConfig { ... };
    // or if filtering is not required:
    // let config = FilterConfig::default();

    // the path to the file where the data types will be written
    let generated_file = "src/api/types.rs";
    // performing the generation
    generate_openapi_types(openapi, config, generated_file).unwrap();
}
```
