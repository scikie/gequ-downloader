//! 主页爬虫模块
//! 
//! 【职责】爬取歌曲客网站主页，提取热门搜索、热门歌手等信息
//! 
//! 【技术选型：scraper】
//! scraper是Rust的HTML解析库，基于Servo的html5ever：
//! - 符合HTML5标准
//! - 支持CSS选择器（类似jQuery）
//! - 内存安全，防止常见的XSS漏洞

use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::path::Path;

// 【知识点：数据结构体设计】
// 每个结构体对应页面中的一个数据实体
// 使用serde派生宏支持JSON序列化

/// 搜索关键词条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKeywordItem {
    pub keyword: String,  // 关键词文本
    pub url: String,      // 链接URL
}

/// 带排名的关键词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedKeyword {
    pub rank: i32,        // 排名序号
    pub keyword: String,
    pub url: String,
}

/// 热门歌手
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSinger {
    pub rank: i32,
    pub name: String,
    pub url: String,
}

/// 主页数据聚合
/// 
/// 【设计模式：数据传输对象（DTO）】
/// 封装一次爬取操作返回的所有数据
/// 便于序列化和后续处理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageData {
    pub latest_searches: Vec<SearchKeywordItem>,
    pub hot_keywords: Vec<RankedKeyword>,
    pub hot_singers: Vec<HotSinger>,
}

/// 主页爬虫
/// 
/// 【知识点：结构体作为服务】
/// 将HTTP客户端和配置封装在结构体中
/// 提供面向对象风格的API
pub struct HomepageCrawler {
    cookie: Option<String>,          // 可选Cookie（用于登录状态）
    user_agent: String,              // User-Agent头
    timeout: f64,                    // 超时时间（秒）
    client: reqwest::Client,         // HTTP客户端（可复用）
}

impl HomepageCrawler {
    /// 创建新的爬虫实例
    /// 
    /// 【知识点：构建者模式变体】
    /// 使用Option参数提供灵活的构造函数
    /// 调用者可以选择性地提供配置
    pub fn new(cookie: Option<String>, user_agent: Option<String>, timeout: Option<f64>) -> Self {
        // unwrap_or_else 提供延迟计算的默认值
        // 与unwrap_or的区别：or_else只在None时执行闭包
        let ua = user_agent.unwrap_or_else(|| 
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()
        );
        let t = timeout.unwrap_or(30.0);

        // 【知识点：HTTP头构造】
        // reqwest使用HeaderMap存储HTTP头
        // HeaderValue::from_str 可能失败（非法字符），此处使用unwrap
        // 因为输入是硬编码的合法字符串
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("User-Agent", reqwest::header::HeaderValue::from_str(&ua).unwrap());
        headers.insert("Accept", reqwest::header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
        ));
        headers.insert("Accept-Language", reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9"));

        // 【知识点：Client构建器模式】
        // reqwest::Client::builder 提供链式配置API
        // - timeout: 设置请求超时
        // - default_headers: 设置默认请求头
        // - redirect: 设置重定向策略
        // - build(): 构造Client（可能失败，但默认配置不会）
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(t))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))  // 最多跟随10次重定向
            .build()
            .unwrap();  // 默认配置不会失败

        Self {
            cookie,
            user_agent: ua,
            timeout: t,
            client,
        }
    }

    /// 获取Cookie
    fn get_cookies(&self) -> Option<String> {
        self.cookie.clone()
    }

    /// 异步获取主页HTML
    /// 
    /// 【知识点：异步HTTP请求】
    /// - .get() 创建GET请求构建器
    /// - .header() 添加/覆盖请求头
    /// - .send().await 发送请求并异步等待响应
    /// - .text().await 读取响应体为字符串
    /// 
    /// 【知识点： anyhow::Context】
    /// .context() 为错误添加描述，形成错误链
    pub async fn get_homepage(&self) -> Result<Html> {
        let mut request = self.client.get("https://www.gequke.com/");
        
        // 如有Cookie则添加
        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("请求主页失败")?;
        let text = resp.text().await.context("读取主页内容失败")?;
        
        // 解析HTML字符串为DOM树
        Ok(Html::parse_document(&text))
    }

    /// 从本地文件加载HTML（用于测试/离线开发）
    /// 
    /// 【知识点：关联函数（静态方法）】
    /// 第一个参数不是&self，通过类型名调用：HomepageCrawler::get_homepage_from_file()
    pub fn get_homepage_from_file(filepath: &str) -> Result<Html> {
        let content = std::fs::read_to_string(Path::new(filepath))
            .context("读取本地HTML文件失败")?;
        Ok(Html::parse_document(&content))
    }

    /// 提取最新搜索关键词
    /// 
    /// 【知识点：CSS选择器】
    /// Selector::parse() 编译CSS选择器字符串
    /// 选择器语法与浏览器一致：
    /// - "div.ilingku_singerlist a" - class为ilingku_singerlist的div内的所有a标签
    /// 
    /// 【知识点：unwrap()的合理使用】
    /// 此处Selector::parse使用unwrap，因为选择器是硬编码的合法字符串
    /// 如果selectors来自用户输入，应该使用Result处理
    pub fn extract_latest_searches(&self, doc: &Html) -> Vec<SearchKeywordItem> {
        let mut results = Vec::new();
        
        let selector = Selector::parse("div.ilingku_singerlist a").unwrap();
        
        // 【知识点：迭代器处理】
        // doc.select() 返回匹配元素的迭代器
        // element.value() 获取元素的底层数据
        // .attr("href") 获取属性值
        for element in doc.select(&selector) {
            // element.text() 返回文本节点的迭代器
            // .collect::<String>() 收集为字符串
            // .trim() 去除首尾空白
            let keyword = element.text().collect::<String>().trim().to_string();
            let url = element.value().attr("href").unwrap_or("").to_string();
            results.push(SearchKeywordItem { keyword, url });
        }

        results
    }

    /// 提取热门搜索关键词（带排名）
    /// 
    /// 【知识点：复杂选择器链】
    /// 页面结构复杂，需要多级选择器定位目标元素
    /// 
    /// 【知识点：标签生命周期】
    /// 'outer 是显式生命周期标签
    /// 用于在嵌套循环中break到外层循环
    pub fn extract_hot_keywords(&self, doc: &Html) -> Vec<RankedKeyword> {
        let mut results = Vec::new();

        // 预编译所有需要的CSS选择器
        let card_selector = Selector::parse("div.card").unwrap();
        let card_body_selector = Selector::parse("div.card-body").unwrap();
        let table_selector = Selector::parse("table.table").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();

        // 'outer 标签用于从嵌套循环中跳出
        'outer: for card in doc.select(&card_selector) {
            if let Some(card_body) = card.select(&card_body_selector).next() {
                // 查找包含"大家都在搜"标题的卡片
                for h6 in card_body.select(&Selector::parse("h6").unwrap()) {
                    let text = h6.text().collect::<String>();
                    if text.contains("大家都在搜") {
                        // 找到表格并提取数据
                        if let Some(table) = card_body.select(&table_selector).next() {
                            if let Some(tbody) = table.select(&tbody_selector).next() {
                                for row in tbody.select(&row_selector) {
                                    let badge_sel = Selector::parse("span.badge").unwrap();
                                    let link_sel = Selector::parse("a").unwrap();
                                    
                                    let rank_badge = row.select(&badge_sel).next();
                                    let keyword_link = row.select(&link_sel).next();

                                    // 解构匹配：同时提取排名和关键词
                                    if let (Some(badge), Some(link)) = (rank_badge, keyword_link) {
                                        let rank_text = badge.text().collect::<String>().trim().to_string();
                                        let rank: i32 = rank_text.parse().unwrap_or(0);
                                        let keyword = link.text().collect::<String>().trim().to_string();
                                        let url = link.value().attr("href").unwrap_or("").to_string();
                                        results.push(RankedKeyword { rank, keyword, url });
                                    }
                                }
                            }
                        }
                        // 找到目标卡片，跳出外层循环
                        break 'outer;
                    }
                }
            }
        }

        results
    }

    /// 提取热门歌手榜
    /// 
    /// 【代码组织】
    /// 与extract_hot_keywords结构类似，处理不同数据
    /// 这种重复是故意的：保持每个方法独立、清晰
    pub fn extract_hot_singers(&self, doc: &Html) -> Vec<HotSinger> {
        let mut results = Vec::new();

        let card_selector = Selector::parse("div.card").unwrap();
        let card_body_selector = Selector::parse("div.card-body").unwrap();
        let table_selector = Selector::parse("table.table").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();

        'outer: for card in doc.select(&card_selector) {
            if let Some(card_body) = card.select(&card_body_selector).next() {
                // 查找包含"热门歌手榜"标题的卡片
                for h6 in card_body.select(&Selector::parse("h6").unwrap()) {
                    let text = h6.text().collect::<String>();
                    if text.contains("热门歌手榜") {
                        if let Some(table) = card_body.select(&table_selector).next() {
                            if let Some(tbody) = table.select(&tbody_selector).next() {
                                for row in tbody.select(&row_selector) {
                                    let badge_sel = Selector::parse("span.badge").unwrap();
                                    let link_sel = Selector::parse("a").unwrap();
                                    
                                    let rank_badge = row.select(&badge_sel).next();
                                    let singer_link = row.select(&link_sel).next();

                                    if let (Some(badge), Some(link)) = (rank_badge, singer_link) {
                                        let rank_text = badge.text().collect::<String>().trim().to_string();
                                        let rank: i32 = rank_text.parse().unwrap_or(0);
                                        let name = link.text().collect::<String>().trim().to_string();
                                        let url = link.value().attr("href").unwrap_or("").to_string();
                                        results.push(HotSinger { rank, name, url });
                                    }
                                }
                            }
                        }
                        break 'outer;
                    }
                }
            }
        }

        results
    }

    /// 提取所有数据
    /// 
    /// 【知识点：组合方法】
    /// 将多个提取方法组合，提供一站式接口
    /// 调用者可以选择使用组合方法或单个方法
    pub fn extract_all(&self, doc: &Html) -> HomepageData {
        HomepageData {
            latest_searches: self.extract_latest_searches(doc),
            hot_keywords: self.extract_hot_keywords(doc),
            hot_singers: self.extract_hot_singers(doc),
        }
    }

    /// 保存数据到JSON文件
    /// 
    /// 【知识点：文件IO错误处理】
    /// 1. 检查并创建父目录
    /// 2. 序列化为格式化的JSON
    /// 3. 写入文件
    /// 每个步骤都有上下文错误信息
    pub fn save_to_json(&self, data: &HomepageData, filepath: &str) -> Result<()> {
        let path = Path::new(filepath);
        
        // 确保输出目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("创建输出目录失败")?;
        }
        
        // serde_json::to_string_pretty 生成带缩进的可读JSON
        let content = serde_json::to_string_pretty(data).context("序列化数据失败")?;
        std::fs::write(path, content).context("写入JSON文件失败")?;
        Ok(())
    }
}

// 【扩展知识：爬虫设计原则】
//
// 1. 分离关注点：
//    - HTTP请求（网络层）
//    - HTML解析（数据层）
//    - 数据存储（持久层）
//
// 2. 容错性：
//    - 使用unwrap_or_default处理解析失败
//    - 选择性提取（即使部分失败也能返回有效数据）
//
// 3. 可测试性：
//    - 支持从文件加载HTML（不依赖网络）
//    - 方法独立，可单独测试每个提取逻辑
//
// 4. 可扩展性：
//    - 新增提取方法不影响现有代码
//    - 数据结构使用Option<T>应对字段缺失
