# rez-next-package

rez-next 的包定义、解析和管理。

## 功能

- 通过 RustPython AST 解析 `package.py`
- 序列化和反序列化（YAML、JSON、Python 格式）
- 包验证
- 变体处理
- 依赖管理

## 用法

```rust
use rez_next_package::{Package, PackageSerializer};
use rez_next_version::Version;

let mut pkg = Package::new("my_package".to_string());
pkg.set_version(Version::parse("1.0.0").unwrap());
pkg.add_requirement("python>=3.8".to_string());

let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
```

## 隶属于 [rez-next](https://github.com/loonghao/rez-next)

许可证：Apache-2.0
