//! # Markdown Translator
//! 
//! 一个高性能的Rust翻译库，专为Markdown文档设计，提供智能文本翻译功能。
//! 
//! ## 主要特性
//! 
//! - **智能代码块处理**: 自动识别并跳过代码块，保持代码完整性
//! - **并行翻译**: 利用Rust异步特性，支持多任务并发翻译
//! - **速率限制**: 内置智能速率限制器，防止API过载
//! - **配置灵活**: 支持TOML配置文件和程序化配置
//! - **错误恢复**: 完善的错误处理和重试机制
//! - **文本分块**: 智能文本分割，处理长文档
//! 
//! ## 快速开始
//! 
//! ```rust
//! use markdown_translator::{TranslationService, TranslationConfig};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = TranslationConfig {
//!         enabled: true,
//!         source_lang: "en".to_string(),
//!         target_lang: "zh".to_string(),
//!         deeplx_api_url: "http://localhost:1188/translate".to_string(),
//!         max_requests_per_second: 1.0,
//!         max_text_length: 3000,
//!         max_paragraphs_per_request: 10,
//!     };
//!     
//!     let translator = TranslationService::new(config);
//!     let result = translator.translate("Hello, world!").await?;
//!     println!("Translation: {}", result);
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## 配置文件支持
//! 
//! ```toml
//! [translation]
//! enabled = true
//! source_lang = "auto"
//! target_lang = "zh"
//! deeplx_api_url = "http://localhost:1188/translate"
//! max_requests_per_second = 1.0
//! max_text_length = 3000
//! max_paragraphs_per_request = 10
//! ```

pub mod config;
pub mod error;
pub mod types;
pub mod translator;

pub use config::TranslationLibConfig;
pub use error::{TranslationError, Result};
pub use types::{
    TranslationConfig, RetryConfig, DeepLXRequest, DeepLXResponse, 
    DpTransRequest, TextSegment
};
pub use translator::{TranslationService, RateLimiter, retry_with_backoff};