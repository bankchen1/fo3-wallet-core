# 贡献指南

感谢您考虑为 FO3 Wallet Core 项目做出贡献！以下是一些指导原则，帮助您参与项目开发。

## 开发流程

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 代码风格

我们使用 Rust 的官方代码风格。请确保您的代码通过 `rustfmt` 格式化：

```bash
cargo fmt
```

并且通过 `clippy` 检查：

```bash
cargo clippy
```

## 测试

请为您的代码添加适当的测试。所有测试应该通过：

```bash
cargo test
```

## 文档

请为您的代码添加适当的文档注释。我们使用标准的 Rust 文档注释格式：

```rust
/// 这是一个函数的文档注释
fn example_function() {
    // 实现...
}
```

## Pull Request 流程

1. 确保您的 PR 描述清楚地说明了您所做的更改
2. 确保所有自动化测试通过
3. 更新相关文档
4. 您的 PR 将由维护者审查，可能会要求进行更改

## 行为准则

请尊重所有项目参与者。我们期望所有贡献者遵循开源社区的最佳实践和礼仪。
