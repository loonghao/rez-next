# rez-next-version

rez-next 的版本解析、比较和范围操作。

## 功能

- 版本字符串解析（`1.2.3`、`1.2.3-alpha.1`）
- 有序比较和排序
- 版本范围解析和包含检查
- 基于状态机的分词器

## 用法

```rust
use rez_next_version::Version;

let v = Version::parse("1.2.3-alpha.1").unwrap();
let v2 = Version::parse("2.0.0").unwrap();
assert!(v < v2);
```

## 隶属于 [rez-next](https://github.com/loonghao/rez-next)

许可证：Apache-2.0
