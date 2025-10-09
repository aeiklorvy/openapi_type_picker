# OpenAPI Type Picker

This library is designed to generate Rust data types from the OpenAPI specification. It is very useful in situations where API is too versatile and has several hundred methods, and your service needs to work with only a small number of them. The library does not generate API calls like it does openapi generator (<https://openapi-generator.tech>), providing freedom of choice of methods and tools.

## Quick start

Let's assume that there is some specification from which it is required to generate only certain types of data (let's call it `schema.json`):
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
        "items": {
          "$ref": "#/components/schemas/Pet"
        }
      },
    }
  }
}
```
And there is also such a configuration file, because I don't need all the fields of all data types (`config.json`):
```json
{
  "include": {     // generate only "Pet" struct
    "Pet": ["id"], // only with "id" property
  }
}
```
In order to generate a file with the definition of data types, in the file `build.rs` need to add a function call:
```rust
use openapi_type_picker::*;
fn main() {
    write_openapi_types(
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
- `auto_include_dependencies`: if `true`, automatically adds schemas to the filter if the fields of another schema refer to it. Default is `false`. See the next chapter for details.

For examples, the petstore demo scheme is used from the very beginning: <https://learn.openapis.org/examples/v3.0/petstore.html>.

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
In fact, this filter will panic, since `Pets` in specification refers to a `Pet` schema that is not present in the filter.

If the schema has only a specific list of properties, then the corresponding structure will still be generated, but without the specified fields (in this example, only `Pet::id` will be generated):
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

## How does automatic dependency inclusion work?

Two concepts should be distinguished: the scheme is *presented* in the filter and the scheme is *included/excluded* in the filter. The first means that the scheme was not added to the filter at all. Roughly speaking, nowhere in the filter will we find a string with the name of such a scheme. The second means that the scheme is found in the filter in inclusions or exclusions. That is, the scheme is presented, and some restrictions apply to it.

By default, `auto_include_dependencies` is disabled (`false`). This means that only the schemes allowed by the filter will be generated, and no others. If a field in one schema refers to another that is not included in the filter, it will cause a generation error.

When the `auto_include_dependencies` is enabled (`true`), the behavior changes: if the scheme is not presented in the filter, it will be automatically included in the list of schemes to generate with its properties. Otherwise, if the scheme is restricted by a filter, then the dependency search will take into account only the fields of this scheme that match the filter.

Let's take a small filter example and analyze what `auto_include_dependencies` does.
```json
{
  "include": {
    "Pets": "*",
  }
}
```
If `auto_include_dependencies = false` (default), the generator will return an error because it will not be able to build the correct Rust code:
```rust
type Pets = Vec<Pet>;
// error: there is no `Pet` type
```
To solve this problem, you can either include the missing schemas in the filter, or activate the `auto_include_dependencies` setting. If `auto_include_dependencies = true`, then the generator will automatically detect that `Pets` refers to `Pet` and generate it too:
```rust
type Pets = Vec<Pet>;
struct Pet { ... }
// success: `Pet` is defined
```

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
    write_openapi_types(openapi, config, generated_file).unwrap();
}
```
