//! 错误处理模块
//! 
//! 定义翻译库中使用的错误类型和错误处理机制。

use std::fmt;

/// 翻译错误类型
/// 
/// 包含翻译过程中可能出现的各种错误情况。
/// 
/// # 变体说明
/// 
/// * `Http` - HTTP请求错误
/// * `Custom` - 自定义错误消息
/// * `RateLimitError` - 速率限制错误
/// * `ApiError` - API响应错误，包含错误代码和消息
/// * `ParseError` - 解析错误
#[derive(Debug)]
pub enum TranslationError {
    /// HTTP请求错误
    Http(reqwest::Error),
    /// 自定义错误消息
    Custom(String),
    /// 速率限制错误
    RateLimitError(String),
    /// API响应错误
    ApiError { 
        /// 错误代码
        code: i32, 
        /// 错误消息
        message: String 
    },
    /// 解析错误
    ParseError(String),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationError::Http(e) => write!(f, "HTTP error: {}", e),
            TranslationError::Custom(msg) => write!(f, "{}", msg),
            TranslationError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
            TranslationError::ApiError { code, message } => write!(f, "API error {}: {}", code, message),
            TranslationError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for TranslationError {}

impl From<reqwest::Error> for TranslationError {
    fn from(error: reqwest::Error) -> Self {
        TranslationError::Http(error)
    }
}

impl From<String> for TranslationError {
    fn from(error: String) -> Self {
        TranslationError::Custom(error)
    }
}

impl From<&str> for TranslationError {
    fn from(error: &str) -> Self {
        TranslationError::Custom(error.to_string())
    }
}

/// 翻译结果类型别名
/// 
/// 简化返回类型，使用 `TranslationError` 作为错误类型。
/// 
/// # 示例
/// 
/// ```rust
/// use markdown_translator::{Result, TranslationError};
/// 
/// fn example_function() -> Result<String> {
///     Ok("Success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, TranslationError>;