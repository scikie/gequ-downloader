//! 配置管理模块
//! 
//! 【设计模式：配置对象模式（Configuration Object Pattern）】
//! 将所有配置项封装在一个结构体中，提供统一的加载、保存、访问接口
//! 优点：
//! - 集中管理配置，避免分散的魔法字符串
//! - 类型安全，编译期检查配置项类型
//! - 支持默认值，简化用户配置

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// 【知识点：cfg目录】
// dirs::config_dir() 返回操作系统的配置目录：
// - Windows: %APPDATA% (如 C:\Users\用户名\AppData\Roaming)
// - macOS: ~/Library/Application Support
// - Linux: ~/.config (遵循XDG规范)
// 这是存放应用程序配置文件的标准位置

/// 应用程序配置
/// 
/// 【知识点：derive宏】
/// Debug: 支持 {:?} 格式化输出
/// Clone: 支持显式深拷贝
/// Serialize/Deserialize: 支持JSON序列化/反序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GequConfig {
    /// Cookie字符串，用于需要登录的页面
    /// 
    /// 【安全提示】
    /// 生产环境应考虑：
    /// - 使用密钥管理服务存储敏感信息
    /// - 配置文件设置适当权限（如600）
    /// - 支持从环境变量读取
    pub cookie: String,
    
    /// 数据库文件路径
    pub db_path: String,
    
    /// 下载目录
    pub download_dir: String,
    
    /// 输出格式（table/json）
    pub output_format: String,
    
    /// HTTP请求User-Agent头
    pub user_agent: String,
    
    /// HTTP请求超时（秒）
    pub timeout: f64,
}

// 【知识点：Default Trait】
// 为类型提供默认值，通常与结构体字段顺序一致
// 可以用 #[derive(Default)] 自动生成，但这里需要自定义值
impl Default for GequConfig {
    /// 提供默认配置
    /// 
    /// 【知识点：构造函数模式】
    /// Default::default() 是Rust中获取默认值的惯用方式
    /// 与结构体更新语法结合使用：
    /// let config = GequConfig { cookie: "xxx".to_string(), ..Default::default() };
    fn default() -> Self {
        Self {
            cookie: String::new(),
            db_path: "gequke.db".to_string(),
            download_dir: "downloads".to_string(),
            output_format: "table".to_string(),
            // 模拟主流浏览器的User-Agent，避免被识别为爬虫
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
            timeout: 30.0,
        }
    }
}

impl GequConfig {
    /// 获取配置目录
    /// 
    /// 【知识点：PathBuf】
    /// PathBuf 是Rust的可变路径类型，类似String与&str的关系
    /// - 拥有路径数据的所有权
    /// - 支持跨平台路径操作（/ vs \）
    /// - 方法：join() 连接路径，push() 追加组件
    /// 
    /// 【错误处理】
    /// unwrap_or_else 在失败时提供默认值，这里回退到当前目录
    pub fn get_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gequ")
    }

    /// 获取配置文件完整路径
    pub fn get_config_file() -> PathBuf {
        // 链式调用：先获取配置目录，再连接文件名
        Self::get_config_dir().join("config.json")
    }

    /// 从文件加载配置
    /// 
    /// 【知识点：惰性初始化】
    /// 如果配置文件不存在，返回默认配置而不是错误
    /// 这是"约定优于配置"理念的体现
    /// 
    /// 【anyhow::Context】
    /// .context() 为错误添加描述性上下文，便于调试
    /// 错误信息会显示："解析配置文件失败: [原始错误]"
    pub fn load() -> Result<Self> {
        let config_file = Self::get_config_file();
        
        if config_file.exists() {
            // 【知识点：? 操作符与错误转换】
            // std::io::Error 通过 From trait 自动转换为 anyhow::Error
            let content = std::fs::read_to_string(&config_file)
                .context("读取配置文件失败")?;
            
            // serde_json::Error 同样自动转换
            let config: GequConfig = serde_json::from_str(&content)
                .context("解析配置文件失败")?;
            Ok(config)
        } else {
            // 配置文件不存在，返回默认值
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    /// 
    /// 【知识点：错误传播与上下文】
    /// 每个操作都添加 .context()，形成完整的错误链：
    /// "写入配置文件失败: 序列化配置失败: ..."
    pub fn save(&self) -> Result<()> {
        let config_file = Self::get_config_file();
        
        // 递归创建父目录（如果不存在）
        // parent() 返回 Option<&Path>，unwrap() 假设路径一定有父目录
        std::fs::create_dir_all(config_file.parent().unwrap())
            .context("创建配置目录失败")?;
        
        // to_string_pretty 生成带缩进的JSON，便于人工编辑
        let content = serde_json::to_string_pretty(self)
            .context("序列化配置失败")?;
        
        std::fs::write(&config_file, content)
            .context("写入配置文件失败")?;
        Ok(())
    }

    /// 获取配置项值
    /// 
    /// 【知识点：&str参数】
    /// 使用 &str 而非 String 作为参数：
    /// - 接受字符串字面量和String引用
    /// - 不获取所有权，不分配新内存
    /// - 更符合Rust的零成本抽象理念
    /// 
    /// 【设计模式：String键访问】
    /// 提供字符串键访问，便于CLI动态查询
    /// 生产环境建议使用枚举键以获得类型安全
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "cookie" => Some(self.cookie.clone()),
            "db_path" => Some(self.db_path.clone()),
            "download_dir" => Some(self.download_dir.clone()),
            "output_format" => Some(self.output_format.clone()),
            "user_agent" => Some(self.user_agent.clone()),
            "timeout" => Some(self.timeout.to_string()),
            _ => None,  // 未知键返回None
        }
    }

    /// 设置配置项值并自动保存
    /// 
    /// 【知识点：&mut self】
    /// 需要可变引用才能修改结构体字段
    /// Rust的借用规则：
    /// - 同一时刻只能有一个可变引用，或多个不可变引用
    /// - 可变引用与不可变引用不能共存
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "cookie" => self.cookie = value.to_string(),
            "db_path" => self.db_path = value.to_string(),
            "download_dir" => self.download_dir = value.to_string(),
            "output_format" => self.output_format = value.to_string(),
            "user_agent" => self.user_agent = value.to_string(),
            "timeout" => {
                // 类型转换失败时返回自定义错误
                self.timeout = value.parse()
                    .context("timeout 必须是数字")?;
            }
            // 未知键返回错误
            _ => return Err(anyhow::anyhow!("未知配置项: {}", key)),
        }
        // 修改后自动持久化
        self.save()?;
        Ok(())
    }

    /// 重置为默认配置
    /// 
    /// 【知识点：解引用赋值】
    /// *self = Self::default() 使用DerefMut解引用后赋值
    /// 这会完全替换self指向的值
    pub fn reset(&mut self) -> Result<()> {
        *self = Self::default();
        self.save()?;
        Ok(())
    }
}

// 【扩展知识：配置管理进阶】
// 
// 1. 配置验证
// impl GequConfig {
//     pub fn validate(&self) -> Result<()> {
//         if self.timeout <= 0.0 {
//             return Err(anyhow::anyhow!("timeout必须为正数"));
//         }
//         // ...更多验证
//         Ok(())
//     }
// }
//
// 2. 环境变量覆盖
// pub fn load_with_env_override() -> Result<Self> {
//     let mut config = Self::load()?;
//     if let Ok(cookie) = std::env::var("GEQU_COOKIE") {
//         config.cookie = cookie;
//     }
//     Ok(config)
// }
//
// 3. 配置文件热重载
// 使用 notify crate 监听文件变化
