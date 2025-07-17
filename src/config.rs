//! 配置管理模块
//! 
//! 提供TOML配置文件的读取、写入和自动发现功能。

use crate::types::TranslationConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 翻译库配置结构
/// 
/// 包含所有翻译相关的配置选项，支持从TOML文件加载和保存。
/// 
/// # 示例
/// 
/// ```rust
/// use markdown_translator::TranslationLibConfig;
/// 
/// // 从默认位置加载配置
/// let config = TranslationLibConfig::load_from_default_locations();
/// 
/// // 从指定文件加载配置
/// let config = TranslationLibConfig::from_file("config.toml").unwrap();
/// 
/// // 保存配置到文件
/// config.save_to_file("output.toml").unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationLibConfig {
    /// 翻译配置
    #[serde(default)]
    pub translation: TranslationConfig,
}

impl Default for TranslationLibConfig {
    fn default() -> Self {
        Self {
            translation: TranslationConfig::default(),
        }
    }
}

impl TranslationLibConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: TranslationLibConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from multiple possible locations
    pub fn load_from_default_locations() -> Self {
        let possible_paths = [
            "translation-config.toml",
            "config.toml",
            ".translation-config.toml",
        ];

        for path in &possible_paths {
            if Path::new(path).exists() {
                match Self::from_file(path) {
                    Ok(config) => {
                        println!("Loaded configuration from: {}", path);
                        return config;
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load config from {}: {}", path, e);
                    }
                }
            }
        }

        println!("No configuration file found, using defaults");
        Self::default()
    }

    /// Generate example configuration file
    pub fn generate_example_config<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let example_config = Self::default();
        example_config.save_to_file(path)?;
        Ok(())
    }
}