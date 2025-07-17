//! 类型定义模块
//! 
//! 定义翻译库中使用的所有数据结构和配置类型。

use serde::{Deserialize, Serialize};

/// 翻译配置
/// 
/// 包含翻译服务的所有配置选项，如API地址、语言设置、性能参数等。
/// 
/// # 字段说明
/// 
/// * `enabled` - 是否启用翻译功能
/// * `source_lang` - 源语言代码，"auto"表示自动检测
/// * `target_lang` - 目标语言代码
/// * `deeplx_api_url` - DeepLX API地址
/// * `max_requests_per_second` - 每秒最大请求数
/// * `max_text_length` - 单次翻译的最大文本长度
/// * `max_paragraphs_per_request` - 单次请求的最大段落数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    /// 是否启用翻译功能
    pub enabled: bool,
    /// 源语言代码，"auto"表示自动检测
    pub source_lang: String,
    /// 目标语言代码
    pub target_lang: String,
    /// DeepLX API地址
    pub deeplx_api_url: String,
    /// 每秒最大请求数
    pub max_requests_per_second: f64,
    /// 单次翻译的最大文本长度
    pub max_text_length: usize,
    /// 单次请求的最大段落数
    pub max_paragraphs_per_request: usize,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            source_lang: "auto".to_string(),
            target_lang: "zh".to_string(),
            deeplx_api_url: "http://localhost:1188/translate".to_string(),
            max_requests_per_second: 0.5,
            max_text_length: 3000,
            max_paragraphs_per_request: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 1,  // 减少重试次数
            initial_delay_ms: 100,  // 减少初始延迟
            max_delay_ms: 1000,  // 减少最大延迟
            backoff_multiplier: 1.2,  // 减少退避倍数
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeepLXRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DpTransRequest {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
}

#[derive(Debug, Deserialize)]
pub struct DeepLXResponse {
    pub code: i32,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct TextSegment {
    pub content: String,
    pub is_code_block: bool,
}