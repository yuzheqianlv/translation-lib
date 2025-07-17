//! 翻译服务核心模块
//! 
//! 提供主要的翻译功能，包括并行处理、速率限制和智能文本分块。

use crate::types::{TranslationConfig, DeepLXRequest, DeepLXResponse, DpTransRequest, RetryConfig, TextSegment};
use crate::error::{Result, TranslationError};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

/// 速率限制器
/// 
/// 用于控制API请求频率，防止超出服务提供商的速率限制。
/// 支持并发请求和自适应延迟。
#[derive(Clone)]
pub struct RateLimiter {
    /// 信号量，用于控制并发请求数量
    semaphore: Arc<Semaphore>,
    /// 请求间隔延迟
    delay: Duration,
}

impl RateLimiter {
    /// 创建新的速率限制器
    /// 
    /// # 参数
    /// 
    /// * `requests_per_second` - 每秒允许的最大请求数
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use markdown_translator::RateLimiter;
    /// 
    /// let limiter = RateLimiter::new(1.0); // 每秒1个请求
    /// ```
    pub fn new(requests_per_second: f64) -> Self {
        // 允许更多并发，减少延迟
        let permits = (requests_per_second * 2.0).ceil() as usize;
        let delay = Duration::from_millis((500.0 / requests_per_second) as u64); // 减少延迟

        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            delay,
        }
    }

    /// 获取请求许可
    /// 
    /// 在发起API请求前调用此方法，确保不超过配置的速率限制。
    /// 
    /// # 返回
    /// 
    /// * `Ok(())` - 成功获取许可
    /// * `Err(TranslationError)` - 获取许可失败
    pub async fn acquire(&self) -> Result<()> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| TranslationError::RateLimitError(format!("Rate limiter error: {}", e)))?;
        // 在并发环境下减少固定延迟
        if self.delay > Duration::from_millis(100) {
            sleep(self.delay).await;
        }
        Ok(())
    }
}

/// 带指数退避的重试机制
/// 
/// 为API调用提供可靠的重试机制，在失败时按指数增长的延迟重试。
/// 
/// # 参数
/// 
/// * `operation` - 要执行的异步操作
/// * `config` - 重试配置
/// * `rate_limiter` - 速率限制器
/// 
/// # 返回
/// 
/// * `Ok(T)` - 操作成功的结果
/// * `Err(TranslationError)` - 所有重试尝试失败后的错误
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    config: &RetryConfig,
    rate_limiter: &RateLimiter,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = config.initial_delay_ms;

    for attempt in 0..=config.max_retries {
        rate_limiter.acquire().await?;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt == config.max_retries => return Err(e),
            Err(e) => {
                eprintln!("Attempt {} failed: {}. Retrying in {}ms...", attempt + 1, e, delay);
                sleep(Duration::from_millis(delay)).await;
                delay = std::cmp::min(
                    (delay as f64 * config.backoff_multiplier) as u64,
                    config.max_delay_ms,
                );
            }
        }
    }

    unreachable!()
}

/// 翻译服务主类
/// 
/// 提供完整的翻译功能，包括文本分块、并行处理、代码块跳过等高级特性。
/// 支持多种翻译API，内置速率限制和错误恢复机制。
/// 
/// # 示例
/// 
/// ```rust
/// use markdown_translator::{TranslationService, TranslationConfig};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TranslationConfig::default();
///     let service = TranslationService::new(config);
///     
///     let result = service.translate("Hello, world!").await?;
///     println!("Translation: {}", result);
///     
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct TranslationService {
    /// HTTP客户端，用于API调用
    client: Client,
    /// 速率限制器
    rate_limiter: RateLimiter,
    /// 翻译配置
    config: TranslationConfig,
}

impl TranslationService {
    /// 创建新的翻译服务实例
    /// 
    /// # 参数
    /// 
    /// * `config` - 翻译配置，包含API地址、语言设置等
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use markdown_translator::{TranslationService, TranslationConfig};
    /// 
    /// let config = TranslationConfig {
    ///     enabled: true,
    ///     source_lang: "en".to_string(),
    ///     target_lang: "zh".to_string(),
    ///     deeplx_api_url: "http://localhost:1188/translate".to_string(),
    ///     max_requests_per_second: 1.0,
    ///     max_text_length: 3000,
    ///     max_paragraphs_per_request: 10,
    /// };
    /// 
    /// let service = TranslationService::new(config);
    /// ```
    pub fn new(config: TranslationConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(5)
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .http1_title_case_headers()
            .http2_keep_alive_interval(None)
            .user_agent("Mozilla/5.0 (compatible; MarkdownDownloader/1.0)")
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Failed to create optimized client: {}, using default", e);
                Client::new()
            });
            
        Self {
            client,
            rate_limiter: RateLimiter::new(config.max_requests_per_second),
            config,
        }
    }

    /// 翻译文本
    /// 
    /// 主要的翻译接口，支持智能分块、并行处理和代码块跳过。
    /// 
    /// # 参数
    /// 
    /// * `text` - 要翻译的文本，支持Markdown格式
    /// 
    /// # 返回
    /// 
    /// * `Ok(String)` - 翻译后的文本
    /// * `Err(TranslationError)` - 翻译过程中的错误
    /// 
    /// # 特性
    /// 
    /// - 自动检测并跳过代码块
    /// - 智能文本分块，支持长文档
    /// - 并行处理多个文本块
    /// - 保持Markdown格式
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use markdown_translator::{TranslationService, TranslationConfig};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = TranslationConfig::default();
    ///     let service = TranslationService::new(config);
    ///     
    ///     let markdown = r#"
    ///     # Hello World
    ///     
    ///     This is a markdown document.
    ///     
    ///     ```rust
    ///     fn main() {
    ///         println!("Hello, world!");
    ///     }
    ///     ```
    ///     "#;
    ///     
    ///     let translated = service.translate(markdown).await?;
    ///     println!("{}", translated);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn translate(&self, text: &str) -> Result<String> {
        if !self.config.enabled {
            return Ok(text.to_string());
        }

        println!("文本总长度: {} 字符", text.len());

        if text.len() <= self.config.max_text_length {
            println!("文本较短，直接翻译");
            return self.translate_chunk(text).await;
        }

        let chunks = self.split_text_into_chunks(text);
        println!("文本较长，分为 {} 块进行翻译", chunks.len());

        let mut translated_chunks = Vec::new();

        // 并行处理所有块
        let mut futures = Vec::new();
        
        for (i, chunk) in chunks.iter().enumerate() {
            println!("准备翻译第 {} 块，长度: {} 字符", i + 1, chunk.len());
            
            if self.is_code_block_chunk(chunk) {
                // 代码块直接返回结果
                let result = chunk.strip_prefix("__CODE_BLOCK__").unwrap_or(chunk).to_string();
                futures.push(Box::pin(async move { Ok(result) }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>);
            } else {
                // 翻译任务
                let chunk_clone = chunk.clone();
                let translator_clone = self.clone();
                futures.push(Box::pin(async move {
                    translator_clone.translate_chunk(&chunk_clone).await
                }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>);
            }
        }

        // 并发执行所有翻译任务，但有并发限制
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5)); // 最多5个并发请求
        let mut handles = Vec::new();
        
        for (i, future) in futures.into_iter().enumerate() {
            let semaphore_clone = semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                println!("开始翻译第 {} 块", i + 1);
                let result = future.await;
                println!("完成翻译第 {} 块", i + 1);
                result
            });
            handles.push(handle);
        }

        // 收集所有结果
        for handle in handles {
            let result = handle.await.map_err(|e| TranslationError::Custom(e.to_string()))??;
            translated_chunks.push(result);
        }

        Ok(translated_chunks.join("\n\n"))
    }

    fn split_text_into_chunks(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let max_length = self.config.max_text_length;

        if text.len() <= max_length {
            chunks.push(text.to_string());
            return chunks;
        }

        let protected_sections = self.identify_code_blocks(text);
        let segments = self.split_by_code_blocks(text, &protected_sections);

        let mut current_chunk = String::new();

        for segment in segments {
            if segment.is_code_block {
                // 代码块需要特殊处理 - 直接作为独立块处理，不与其他内容合并
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }
                // 给代码块添加特殊标记，便于后续识别
                chunks.push(format!("__CODE_BLOCK__{}", segment.content));
            } else {
                let paragraphs = self.split_text_by_empty_lines(&segment.content);
                
                for paragraph in paragraphs {
                    if paragraph.trim().is_empty() {
                        continue;
                    }

                    let potential_length = if current_chunk.is_empty() {
                        paragraph.len()
                    } else {
                        current_chunk.len() + 2 + paragraph.len()
                    };

                    if potential_length <= max_length {
                        if !current_chunk.is_empty() {
                            current_chunk.push_str("\n\n");
                        }
                        current_chunk.push_str(&paragraph);
                    } else {
                        if !current_chunk.is_empty() {
                            chunks.push(current_chunk.clone());
                            current_chunk.clear();
                        }

                        if paragraph.len() > max_length {
                            let sub_chunks = self.split_long_paragraph(&paragraph, max_length);
                            chunks.extend(sub_chunks);
                        } else {
                            current_chunk = paragraph;
                        }
                    }
                }
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        if chunks.is_empty() {
            chunks.push(text.to_string());
        }

        chunks
    }

    fn identify_code_blocks(&self, text: &str) -> Vec<(usize, usize)> {
        let mut code_blocks = Vec::new();
        let mut in_code_block = false;
        let mut current_start = 0;
        
        let lines: Vec<&str> = text.lines().collect();
        let mut char_pos = 0;
        
        for (_i, line) in lines.iter().enumerate() {
            if line.starts_with("```") {
                if in_code_block {
                    let end_pos = char_pos + line.len();
                    code_blocks.push((current_start, end_pos));
                    in_code_block = false;
                } else {
                    current_start = char_pos;
                    in_code_block = true;
                }
            }
            char_pos += line.len() + 1;
        }
        
        if in_code_block {
            code_blocks.push((current_start, text.len()));
        }
        
        code_blocks
    }

    fn split_by_code_blocks(&self, text: &str, code_blocks: &[(usize, usize)]) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let mut last_end = 0;
        
        for &(start, end) in code_blocks {
            if start > last_end {
                let content = text[last_end..start].to_string();
                if !content.trim().is_empty() {
                    segments.push(TextSegment {
                        content,
                        is_code_block: false,
                    });
                }
            }
            
            let content = text[start..end].to_string();
            segments.push(TextSegment {
                content,
                is_code_block: true,
            });
            
            last_end = end;
        }
        
        if last_end < text.len() {
            let content = text[last_end..].to_string();
            if !content.trim().is_empty() {
                segments.push(TextSegment {
                    content,
                    is_code_block: false,
                });
            }
        }
        
        if segments.is_empty() {
            segments.push(TextSegment {
                content: text.to_string(),
                is_code_block: false,
            });
        }
        
        segments
    }

    fn split_text_by_empty_lines(&self, text: &str) -> Vec<String> {
        let max_length = self.config.max_text_length;
        
        if text.len() <= max_length {
            return vec![text.to_string()];
        }
        
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut result = Vec::new();
        let mut current_group = Vec::new();
        let mut current_length = 0;
        
        for paragraph in paragraphs {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            
            let para_len = paragraph.len();
            
            let potential_length = if current_group.is_empty() {
                para_len
            } else {
                current_length + 2 + para_len
            };
            
            if potential_length <= max_length {
                current_group.push(paragraph);
                current_length = potential_length;
            } else {
                if !current_group.is_empty() {
                    result.push(current_group.join("\n\n"));
                    current_group.clear();
                }
                
                if para_len > max_length {
                    let sub_parts = self.split_long_paragraph(paragraph, max_length);
                    result.extend(sub_parts);
                    current_length = 0;
                } else {
                    current_group.push(paragraph);
                    current_length = para_len;
                }
            }
        }
        
        if !current_group.is_empty() {
            result.push(current_group.join("\n\n"));
        }
        
        result
    }

    fn split_long_paragraph(&self, paragraph: &str, max_length: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < paragraph.len() {
            let end = std::cmp::min(start + max_length, paragraph.len());
            let mut actual_end = end;

            if end < paragraph.len() {
                for i in (start..end).rev() {
                    let ch = paragraph.chars().nth(i).unwrap_or(' ');
                    if ch == '.' || ch == '!' || ch == '?' || ch == '。' || ch == '！' || ch == '？' {
                        actual_end = i + 1;
                        break;
                    }
                }

                if actual_end == end {
                    for i in (start..end).rev() {
                        let ch = paragraph.chars().nth(i).unwrap_or(' ');
                        if ch == ' ' || ch == '\n' || ch == '\t' {
                            actual_end = i + 1;
                            break;
                        }
                    }
                }

                if actual_end == end && end - start < max_length / 2 {
                    actual_end = end;
                }
            }

            let chunk = paragraph[start..actual_end].trim().to_string();
            if !chunk.is_empty() {
                chunks.push(chunk);
            }

            start = actual_end;
        }

        chunks
    }

    async fn translate_chunk(&self, text: &str) -> Result<String> {
        println!("发送翻译请求到: {}", self.config.deeplx_api_url);
        println!("翻译文本长度: {} 字符", text.len());

        let retry_config = RetryConfig::default();
        let client = &self.client;
        let config = &self.config;
        let text_clone = text.to_string();

        let result = retry_with_backoff(
            || {
                let client = client.clone();
                let config = config.clone();
                let text = text_clone.clone();

                Box::pin(async move {
                    let response = if config.deeplx_api_url.contains("dptrans") {
                        println!("使用dptrans API格式请求");

                        let request = DpTransRequest {
                            text: text.clone(),
                            source_lang: if config.source_lang == "auto" { "auto".to_string() } else { config.source_lang.clone() },
                            target_lang: config.target_lang.clone(),
                        };

                        client
                            .post(&config.deeplx_api_url)
                            .header("Content-Type", "application/json")
                            .header("Accept", "application/json, text/plain, */*")
                            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                            .json(&request)
                            .send()
                            .await
                            .map_err(|e| {
                                TranslationError::Custom(format!("DeepLX网络请求失败: {}", e))
                            })?
                    } else {
                        println!("使用标准DeepLX API格式请求");

                        let request = DeepLXRequest {
                            text: text.clone(),
                            source_lang: config.source_lang.clone(),
                            target_lang: config.target_lang.clone(),
                        };

                        client
                            .post(&config.deeplx_api_url)
                            .header("Content-Type", "application/json")
                            .header("Accept", "application/json")
                            .json(&request)
                            .send()
                            .await
                            .map_err(|e| {
                                TranslationError::Custom(format!("DeepLX网络请求失败: {}", e))
                            })?
                    };

                    let status = response.status();
                    println!("DeepLX响应状态: {}", status);

                    if response.status().is_success() {
                        let response_text = response
                            .text()
                            .await
                            .map_err(|e| TranslationError::Custom(format!("读取响应文本失败: {}", e)))?;

                        if let Ok(result) = serde_json::from_str::<DeepLXResponse>(&response_text) {
                            if result.code == 200 {
                                if result.data.is_empty() {
                                    Err(TranslationError::Custom("DeepLX返回了空的翻译结果".to_string()))
                                } else {
                                    Ok(result.data)
                                }
                            } else {
                                Err(TranslationError::ApiError {
                                    code: result.code,
                                    message: format!("DeepLX翻译失败，返回代码: {}", result.code)
                                })
                            }
                        } else {
                            if response_text.trim().is_empty() {
                                Err(TranslationError::Custom("API返回了空的翻译结果".to_string()))
                            } else if response_text.starts_with("{") {
                                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response_text) {
                                    if let Some(translated) = json_value
                                        .get("translated_text")
                                        .or_else(|| json_value.get("result"))
                                        .or_else(|| json_value.get("translation"))
                                        .or_else(|| json_value.get("data"))
                                        .and_then(|v| v.as_str())
                                    {
                                        Ok(translated.to_string())
                                    } else {
                                        Err(TranslationError::ParseError(format!(
                                            "无法从JSON响应中提取翻译结果: {}",
                                            response_text
                                        )))
                                    }
                                } else {
                                    Err(TranslationError::ParseError(format!("无法解析JSON响应: {}", response_text)))
                                }
                            } else {
                                println!("假设响应是纯文本翻译结果");
                                Ok(response_text)
                            }
                        }
                    } else {
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "无法读取错误信息".to_string());
                        Err(TranslationError::ApiError {
                            code: status.as_u16() as i32,
                            message: format!("DeepLX API请求失败: {} - {}", status, error_text)
                        })
                    }
                })
            },
            &retry_config,
            &self.rate_limiter,
        )
        .await?;

        Ok(result)
    }

    /// 检测chunk是否为代码块
    fn is_code_block_chunk(&self, chunk: &str) -> bool {
        chunk.starts_with("__CODE_BLOCK__") || chunk.trim_start().starts_with("```")
    }
}