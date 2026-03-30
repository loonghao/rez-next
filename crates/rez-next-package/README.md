# rez-next-package

Package definition, parsing, and management for rez-next.

## Features

- `package.py` parsing via RustPython AST
- Serialization and deserialization (YAML, JSON, Python format)
- Package validation
- Variant handling
- Requirement management

## Usage

```rust
use rez_next_package::{Package, PackageSerializer};
use rez_next_version::Version;

let mut pkg = Package::new("my_package".to_string());
pkg.set_version(Version::parse("1.0.0").unwrap());
pkg.add_requirement("python>=3.8".to_string());

let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
```

## Part of [rez-next](https://github.com/loonghao/rez-next)

License: Apache-2.0
