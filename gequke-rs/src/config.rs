use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GequConfig {
    pub cookie: String,
    pub db_path: String,
    pub download_dir: String,
    pub output_format: String,
    pub user_agent: String,
    pub timeout: f64,
}

impl Default for GequConfig {
    fn default() -> Self {
        Self {
            cookie: String::new(),
            db_path: "gequke.db".to_string(),
            download_dir: "downloads".to_string(),
            output_format: "table".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
            timeout: 30.0,
        }
    }
}

impl GequConfig {
    pub fn get_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gequ")
    }

    pub fn get_config_file() -> PathBuf {
        Self::get_config_dir().join("config.json")
    }

    pub fn load() -> Result<Self> {
        let config_file = Self::get_config_file();
        
        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)
                .context("读取配置文件失败")?;
            let config: GequConfig = serde_json::from_str(&content)
                .context("解析配置文件失败")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_file = Self::get_config_file();
        std::fs::create_dir_all(config_file.parent().unwrap())
            .context("创建配置目录失败")?;
        
        let content = serde_json::to_string_pretty(self)
            .context("序列化配置失败")?;
        std::fs::write(&config_file, content)
            .context("写入配置文件失败")?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "cookie" => Some(self.cookie.clone()),
            "db_path" => Some(self.db_path.clone()),
            "download_dir" => Some(self.download_dir.clone()),
            "output_format" => Some(self.output_format.clone()),
            "user_agent" => Some(self.user_agent.clone()),
            "timeout" => Some(self.timeout.to_string()),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "cookie" => self.cookie = value.to_string(),
            "db_path" => self.db_path = value.to_string(),
            "download_dir" => self.download_dir = value.to_string(),
            "output_format" => self.output_format = value.to_string(),
            "user_agent" => self.user_agent = value.to_string(),
            "timeout" => {
                self.timeout = value.parse()
                    .context("timeout 必须是数字")?;
            }
            _ => return Err(anyhow::anyhow!("未知配置项: {}", key)),
        }
        self.save()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        *self = Self::default();
        self.save()?;
        Ok(())
    }
}