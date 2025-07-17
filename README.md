# Markdown Translator

[![Crates.io](https://img.shields.io/crates/v/markdown-translator.svg)](https://crates.io/crates/markdown-translator)
[![Documentation](https://docs.rs/markdown-translator/badge.svg)](https://docs.rs/markdown-translator)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

一个高性能的Rust翻译库，专为Markdown文档设计，提供智能文本翻译功能。

## ✨ 主要特性

- **🚀 高性能并行处理**: 利用Rust异步特性，支持多任务并发翻译
- **🔧 智能代码块处理**: 自动识别并跳过代码块，保持代码完整性
- **⚡ 速率限制**: 内置智能速率限制器，防止API过载
- **📝 配置灵活**: 支持TOML配置文件和程序化配置
- **🔄 错误恢复**: 完善的错误处理和重试机制
- **📄 文本分块**: 智能文本分割，支持长文档翻译
- **🌐 多API支持**: 支持DeepLX和兼容的翻译API

## 🚀 快速开始

### 安装

将以下内容添加到你的 `Cargo.toml`:

```toml
[dependencies]
markdown-translator = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### 基本使用

```rust
use markdown_translator::{TranslationService, TranslationConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 配置翻译服务
    let config = TranslationConfig {
        enabled: true,
        source_lang: "en".to_string(),
        target_lang: "zh".to_string(),
        deeplx_api_url: "http://localhost:1188/translate".to_string(),
        max_requests_per_second: 1.0,
        max_text_length: 3000,
        max_paragraphs_per_request: 10,
    };
    
    // 创建翻译服务
    let translator = TranslationService::new(config);
    
    // 翻译文本
    let markdown = r#"
    # Hello World
    
    This is a markdown document with code:
    
    ```rust
    fn main() {
        println!("Hello, world!");
    }
    ```
    
    The code above will be preserved during translation.
    "#;
    
    let result = translator.translate(markdown).await?;
    println!("Translation result:\n{}", result);
    
    Ok(())
}
```

### 使用配置文件

创建 `translation-config.toml`:

```toml
[translation]
enabled = true
source_lang = "auto"
target_lang = "zh"
deeplx_api_url = "http://localhost:1188/translate"
max_requests_per_second = 2.0
max_text_length = 3000
max_paragraphs_per_request = 10
```

然后在代码中使用:

```rust
use markdown_translator::{TranslationService, TranslationLibConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从配置文件加载配置
    let lib_config = TranslationLibConfig::load_from_default_locations();
    let translator = TranslationService::new(lib_config.translation);
    
    let result = translator.translate("Hello, world!").await?;
    println!("Translation: {}", result);
    
    Ok(())
}
```

## 📋 配置选项

### 翻译配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | `bool` | `false` | 是否启用翻译功能 |
| `source_lang` | `String` | `"auto"` | 源语言代码，"auto"表示自动检测 |
| `target_lang` | `String` | `"zh"` | 目标语言代码 |
| `deeplx_api_url` | `String` | `"http://localhost:1188/translate"` | DeepLX API地址 |
| `max_requests_per_second` | `f64` | `0.5` | 每秒最大请求数 |
| `max_text_length` | `usize` | `3000` | 单次翻译的最大文本长度 |
| `max_paragraphs_per_request` | `usize` | `10` | 单次请求的最大段落数 |

### 配置文件搜索路径

库会按以下顺序搜索配置文件：

1. `translation-config.toml` (当前目录)
2. `config.toml` (当前目录) 
3. `.translation-config.toml` (当前目录)

### 性能调优

#### 高性能配置
适用于本地网络环境或高性能API服务：

```toml
[translation]
enabled = true
source_lang = "auto"
target_lang = "zh"
deeplx_api_url = "http://localhost:1188/translate"
max_requests_per_second = 3.0
max_text_length = 4000
max_paragraphs_per_request = 15
```

#### 稳定配置
适用于公网环境或API限制较严格的场景：

```toml
[translation]
enabled = true
source_lang = "auto"
target_lang = "zh"
deeplx_api_url = "http://localhost:1188/translate"
max_requests_per_second = 0.5
max_text_length = 2000
max_paragraphs_per_request = 5
```

## 🌐 支持的翻译API

### DeepLX

这是推荐的翻译API，支持多种部署方式：

```bash
# 使用Docker运行DeepLX
docker run -d -p 1188:1188 ghcr.io/owo-network/deeplx:latest

# 测试API
curl -X POST http://localhost:1188/translate \
  -H "Content-Type: application/json" \
  -d '{"text":"Hello World","source_lang":"auto","target_lang":"zh"}'
```

### 兼容的API格式

库支持两种API格式：

1. **标准DeepLX格式**:
```json
{
  "text": "Hello, world!",
  "source_lang": "en",
  "target_lang": "zh"
}
```

2. **DpTrans格式**:
```json
{
  "text": "Hello, world!",
  "source_lang": "en", 
  "target_lang": "zh"
}
```

## 🔧 高级特性

### 并行处理

库内置并行处理支持，可以同时翻译多个文本块：

```rust
use markdown_translator::{TranslationService, TranslationConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TranslationConfig {
        enabled: true,
        max_requests_per_second: 3.0, // 增加并发数
        ..Default::default()
    };
    
    let translator = TranslationService::new(config);
    
    // 长文档会自动分块并并行处理
    let long_document = std::fs::read_to_string("long_document.md")?;
    let result = translator.translate(&long_document).await?;
    
    Ok(())
}
```

### 代码块保护

库会自动识别Markdown代码块并跳过翻译：

```markdown
# 标题会被翻译

这段文字会被翻译。

```rust
// 这段代码不会被翻译
fn main() {
    println!("Hello, world!");
}
```

这段文字也会被翻译。
```

### 错误处理和重试

内置重试机制，自动处理临时网络错误：

```rust
use markdown_translator::{TranslationService, TranslationError};

match translator.translate("Hello, world!").await {
    Ok(result) => println!("Translation: {}", result),
    Err(TranslationError::Http(e)) => eprintln!("Network error: {}", e),
    Err(TranslationError::ApiError { code, message }) => {
        eprintln!("API error {}: {}", code, message);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 📊 性能基准

在典型配置下的性能表现：

| 文档大小 | 处理时间 | 并发数 | 吞吐量 |
|----------|----------|--------|--------|
| 1KB | ~1s | 1 | ~1KB/s |
| 10KB | ~3s | 3 | ~3.3KB/s |
| 100KB | ~15s | 5 | ~6.7KB/s |

*注：性能取决于网络延迟、API响应时间和配置参数*

## 🛠️ 开发

### 构建

```bash
cargo build --release
```

### 测试

```bash
cargo test
```

### 文档生成

```bash
cargo doc --open
```

### 运行示例

```bash
# 生成配置文件
cargo run --example generate_config

# 运行翻译测试
cargo run --example test_translation
```

## 🤝 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目使用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [DeepLX](https://github.com/OwO-Network/DeepLX) - 提供免费的翻译API
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP客户端库
- [tokio](https://github.com/tokio-rs/tokio) - 异步运行时
- [serde](https://github.com/serde-rs/serde) - 序列化框架

## 📞 支持

如果遇到问题或有建议，请：

1. 查看[文档](https://docs.rs/markdown-translator)
2. 搜索现有的[Issues](https://github.com/your-username/markdown-translator/issues)
3. 创建新的Issue描述问题

---

**注意**: 请遵守API服务提供商的使用条款和速率限制。合理使用翻译服务，避免滥用。