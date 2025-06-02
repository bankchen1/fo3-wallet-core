# FO3 Wallet Core

A Rust-driven multi-chain wallet and DeFi SDK supporting account management, mnemonic generation, asset synchronization, transaction broadcasting, and DEX contract interactions. This project serves as the core on-chain engine module for the FO3 digital wallet system, designed to be used by multiple platforms (App, Web, Admin).

## 项目实现状态报告

### 代码库实现情况

目前项目已经实现了以下功能：

1. **核心钱包功能**：
   - 助记词生成与管理 (BIP39)
   - 多链密钥派生
   - 交易签名与广播
   - 账户管理

2. **区块链支持**：
   - **以太坊及EVM兼容链**：基本功能已完成，包括交易创建、签名和广播
   - **比特币**：基本功能已完成，包括地址生成和交易签名
   - **Solana**：已完成基础功能，并添加了额外的DeFi和NFT支持

3. **DeFi集成**：
   - **以太坊**：支持Uniswap、Aave和Compound等协议
   - **Solana**：支持Raydium和Orca DEX交易，以及Marinade质押

4. **NFT支持**：
   - Solana NFT查询、元数据获取和转账功能已实现

### 测试情况

1. **单元测试**：
   - 核心功能已有基本单元测试覆盖
   - 密钥派生和助记词生成有完整测试
   - DeFi功能有基本测试用例

2. **集成测试**：
   - 区块链交互测试需要通过环境变量`RUN_SOLANA_TESTS=1`启用
   - CI环境中会自动跳过需要真实RPC连接的测试

3. **测试覆盖率**：
   - 核心加密模块测试覆盖率较高
   - API接口测试覆盖率需要提高

### 编译和构建状态

1. **编译状态**：
   - 项目可以在最新的Rust稳定版(1.70+)上成功编译
   - 使用了特性标志(features)来控制不同区块链的支持
   - 发布构建已优化(LTO, 代码单元优化等)

2. **CI/CD**：
   - GitHub Actions工作流已配置，包括代码检查、测试、格式化和Clippy静态分析
   - 所有PR必须通过CI检查才能合并

### 接口文档

1. **API文档**：
   - REST API接口已在README中列出
   - 详细的Rust API文档可通过`cargo doc --open`生成
   - Solana集成有专门的文档(raydium.md, orca.md, nft.md)

2. **示例代码**：
   - 提供了基本使用示例
   - Solana模块有专门的示例代码

## 项目结构

项目组织为三个主要的crate：

1. `fo3-wallet`: 核心库(lib)，包含：
   - 助记词和私钥管理
   - 多链密钥派生
   - 链上交互
   - 交易签名
   - DeFi协议集成

2. `fo3-wallet-api`: 基于Axum的REST API服务(bin)，提供：
   - 通过HTTP端点暴露钱包核心功能
   - 为客户端应用程序提供清晰的接口

3. `fo3-wallet-solana`: Solana区块链集成库，提供：
   - Solana特定的钱包功能
   - Raydium和Orca DEX集成
   - NFT支持
   - 质押功能

## 环境变量配置

项目使用以下环境变量：

1. **日志配置**：
   - `RUST_LOG`: 控制日志级别，默认为"info,tower_http=debug"

2. **测试配置**：
   - `RUN_SOLANA_TESTS`: 设置为1以启用需要真实RPC连接的Solana测试
   - `CI`: CI环境中自动跳过某些测试

3. **API密钥**：
   - 以太坊RPC需要配置Infura API密钥
   - 目前在代码中硬编码为占位符"your-api-key"，实际使用时需要替换

## 支持的区块链

- 以太坊及EVM兼容链
- Solana
- 比特币

## 功能特性

- **账户管理**：创建、导入和管理使用BIP39助记词的钱包
- **多链支持**：为多个区块链派生地址和密钥
- **交易处理**：创建、签名和广播交易
- **DeFi集成**：与交换、借贷协议和质押平台交互
- **资产管理**：跨链跟踪余额和交易

## 开始使用

### 前提条件

- Rust 1.70+和Cargo
- 加密函数的开发库

### 构建项目

```bash
# 克隆仓库
git clone https://github.com/fo3/fo3-wallet-core.git
cd fo3-wallet-core

# 构建项目
cargo build --release

# 运行API服务器
cargo run -p fo3-wallet-api
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行Solana特定测试(需要网络连接)
RUST_LOG=info RUN_SOLANA_TESTS=1 cargo test
```

## API文档

wallet-api暴露以下端点：

### 钱包管理

- `GET /wallets`: 列出所有钱包
- `POST /wallets`: 创建新钱包
- `POST /wallets/import`: 从助记词导入钱包
- `GET /wallets/:id`: 获取钱包详情
- `PUT /wallets/:id`: 更新钱包
- `DELETE /wallets/:id`: 删除钱包
- `GET /wallets/:id/addresses`: 获取钱包的地址
- `POST /wallets/:id/addresses`: 派生新地址

### 交易

- `GET /transactions`: 列出交易
- `POST /transactions`: 创建新交易
- `GET /transactions/:id`: 获取交易详情
- `POST /transactions/:id/sign`: 签名交易
- `POST /transactions/:id/broadcast`: 广播交易

### DeFi

- `GET /defi/tokens/:address/balance`: 获取代币余额
- `GET /defi/swap/routes`: 获取交换路由
- `POST /defi/swap/execute`: 执行交换
- `GET /defi/lending/markets`: 获取借贷市场
- `GET /defi/lending/positions/:address`: 获取借贷头寸
- `GET /defi/staking/pools`: 获取质押池
- `GET /defi/staking/positions/:address`: 获取质押头寸

### Solana特定API (当启用solana特性时)

- `GET /nft/:wallet_address`: 获取钱包拥有的NFT
- `GET /nft/:mint/metadata`: 获取NFT元数据
- `POST /nft/transfer`: 转移NFT
- `POST /nft/mint`: 铸造新NFT
- `GET /defi/swap/raydium/pairs`: 获取Raydium支持的代币对
- `POST /defi/swap/raydium/quote`: 获取Raydium交换报价
- `POST /defi/swap/raydium/execute`: 执行Raydium交换
- `GET /defi/swap/orca/pairs`: 获取Orca支持的代币对
- `POST /defi/swap/orca/quote`: 获取Orca交换报价
- `POST /defi/swap/orca/execute`: 执行Orca交换

## 未来计划

1. **短期目标**：
   - 完善测试覆盖率
   - 添加更多DeFi协议支持
   - 改进错误处理和日志记录
   - 添加更多文档和示例

2. **中期目标**：
   - WebAssembly (WASM) 支持，用于浏览器集成
   - 硬件钱包集成
   - 增强的安全功能
   - 更多区块链支持

3. **长期目标**：
   - 跨链交易支持
   - 高级DeFi策略
   - 移动SDK集成
   - 企业级功能

## 许可证

MIT License
