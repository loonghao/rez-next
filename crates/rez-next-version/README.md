# rez-next-version

Version parsing, comparison, and range operations for rez-next.

## Features

- Version string parsing (`1.2.3`, `1.2.3-alpha.1`)
- Ordered comparison and sorting
- Version range parsing and containment checks
- State-machine based tokenizer

## Usage

```rust
use rez_next_version::Version;

let v = Version::parse("1.2.3-alpha.1").unwrap();
let v2 = Version::parse("2.0.0").unwrap();
assert!(v < v2);
```

## Part of [rez-next](https://github.com/loonghao/rez-next)

License: Apache-2.0
