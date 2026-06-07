//! 排行榜爬虫模块
//!
//! 【职责】爬取各类排行榜（歌手榜、飙升榜、新歌榜等）
//!
//! 【特殊处理】
//! 歌手榜和其他榜单的页面结构不同：
//! - 歌手榜显示歌手头像和名称
//! - 其他榜单显示歌曲封面、标题、歌手

use anyhow::{Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 歌曲排行条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongRank {
    pub rank: i32,
    pub title: String,
    pub artist: String,
    pub song_id: i64,      // 歌曲唯一ID（从URL中提取）
    pub cover_url: String, // 封面图片URL
    pub song_url: String,  // 歌曲详情页URL
}

/// 歌手排行条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingerRank {
    pub rank: i32,
    pub name: String,
    pub avatar_url: String, // 头像URL
    pub songs_url: String,  // 歌手歌曲列表URL
}

/// 分页信息
///
/// 【设计模式：分页数据传输对象】
/// 封装分页相关的所有元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub current_page: i32,
    pub total_pages: i32,
    pub total_songs: i32,
    pub has_prev: bool,
    pub has_next: bool,
    pub first_page_url: Option<String>,
    pub prev_page_url: Option<String>,
    pub next_page_url: Option<String>,
    pub last_page_url: Option<String>,
}

/// 排行榜页面数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingPageData {
    pub ranking_name: String,     // 榜单名称（如"飙升榜"）
    pub songs: Vec<SongRank>,     // 歌曲列表（非歌手榜）
    pub singers: Vec<SingerRank>, // 歌手列表（歌手榜）
    pub pagination: Pagination,   // 分页信息
}

/// 排行榜爬虫
pub struct RankingCrawler {
    cookie: Option<String>,
    user_agent: String,
    timeout: f64,
    client: reqwest::Client,
    // 【知识点：HashMap存储配置】
    // 使用HashMap存储榜单类型映射，支持O(1)查询
    ranking_types: HashMap<&'static str, &'static str>,
}

impl RankingCrawler {
    /// 创建爬虫实例
    pub fn new(cookie: Option<String>, user_agent: Option<String>, timeout: Option<f64>) -> Self {
        let ua = user_agent.unwrap_or_else(|| {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()
        });
        let t = timeout.unwrap_or(30.0);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "User-Agent",
            reqwest::header::HeaderValue::from_str(&ua).unwrap(),
        );
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("text/html,*/*;q=0.8"),
        );
        headers.insert(
            "Accept-Language",
            reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9"),
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(t))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

        // 初始化榜单类型映射
        let mut ranking_types = HashMap::new();
        ranking_types.insert("singer", "歌手榜");
        ranking_types.insert("surge", "飙升榜");
        ranking_types.insert("new", "新歌榜");
        ranking_types.insert("douyin", "抖音榜");
        ranking_types.insert("jingdian", "怀旧榜");
        ranking_types.insert("dianyin", "电音榜");
        ranking_types.insert("wwdj", "DJ榜");

        Self {
            cookie,
            user_agent: ua,
            timeout: t,
            client,
            ranking_types,
        }
    }

    /// 获取支持的榜单类型
    pub fn get_ranking_types(&self) -> &HashMap<&'static str, &'static str> {
        &self.ranking_types
    }

    fn get_cookies(&self) -> Option<String> {
        self.cookie.clone()
    }

    /// 获取排行榜页面
    ///
    /// 【知识点：URL构造逻辑】
    /// 不同榜单类型有不同的URL模式：
    /// - 歌手榜: /singer/ 或 /singer/{page}
    /// - 其他: /top/{type} 或 /top/{type}?page={page}
    pub async fn get_ranking_page(&self, ranking_type: &str, page: i32) -> Result<Html> {
        // 验证榜单类型是否有效
        if !self.ranking_types.contains_key(ranking_type) {
            return Err(anyhow::anyhow!("无效的榜单类型: {}", ranking_type));
        }

        // 根据类型构造URL
        let url = if ranking_type == "singer" {
            if page > 1 {
                format!("https://www.gequke.com/singer/{}", page)
            } else {
                "https://www.gequke.com/singer/".to_string()
            }
        } else {
            let base = format!("https://www.gequke.com/top/{}", ranking_type);
            if page > 1 {
                format!("{}?page={}", base, page)
            } else {
                base
            }
        };

        let mut request = self.client.get(&url);

        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("请求排行榜页面失败")?;
        let text = resp.text().await.context("读取排行榜内容失败")?;
        Ok(Html::parse_document(&text))
    }

    /// 从本地文件加载
    pub fn get_ranking_page_from_file(filepath: &str) -> Result<Html> {
        let content =
            std::fs::read_to_string(Path::new(filepath)).context("读取本地HTML文件失败")?;
        Ok(Html::parse_document(&content))
    }

    /// 提取榜单名称和总条目数
    ///
    /// 【知识点：正则表达式提取】
    /// 使用regex crate从文本中提取结构化数据
    /// r"(.+?)\s*\(共(\d+)条\)" 匹配：榜单名称（共XX条）
    ///
    /// 返回元组：(名称, 总条目数)
    pub fn extract_ranking_name(&self, doc: &Html) -> (String, i32) {
        let selector = Selector::parse("h1.text-light").unwrap();
        if let Some(h1) = doc.select(&selector).next() {
            let text = h1.text().collect::<String>().trim().to_string();
            let re = Regex::new(r"(.+?)\s*\(共(\d+)条\)").unwrap();
            if let Some(caps) = re.captures(&text) {
                return (caps[1].to_string(), caps[2].parse().unwrap_or(0));
            }
            return (text, 0);
        }
        ("未知榜单".to_string(), 0)
    }

    /// 提取歌曲列表
    ///
    /// 【知识点：表格解析】
    /// 页面使用表格布局，按行和列提取数据
    /// 注意：cols.len() < 4时跳过，处理可能的空行
    pub fn extract_songs(&self, doc: &Html) -> Vec<SongRank> {
        let mut results = Vec::new();

        let table_selector = Selector::parse("table#myTable").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let col_selector = Selector::parse("td").unwrap();

        if let Some(table) = doc.select(&table_selector).next() {
            if let Some(tbody) = table.select(&tbody_selector).next() {
                for row in tbody.select(&row_selector) {
                    // 获取所有单元格
                    let cols: Vec<_> = row.select(&col_selector).collect();
                    if cols.len() < 4 {
                        continue; // 数据不完整，跳过
                    }

                    // 第1列：排名
                    let rank_text = cols[0].text().collect::<String>().trim().to_string();
                    let rank: i32 = rank_text.parse().unwrap_or(0);

                    // 第2列：封面图片
                    let img = cols[1].select(&Selector::parse("img").unwrap()).next();
                    let cover_url = img
                        .map(|i| i.value().attr("src").unwrap_or("").to_string())
                        .unwrap_or_default();

                    // 第3列：歌曲标题和链接
                    let song_link = cols[2].select(&Selector::parse("a").unwrap()).next();
                    let title = song_link
                        .map(|l| l.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();
                    let song_url = song_link
                        .map(|l| l.value().attr("href").unwrap_or("").to_string())
                        .unwrap_or_default();

                    // 【知识点：从URL提取ID】
                    // 歌曲URL格式：/song/{id}
                    // 使用正则提取数字部分
                    let song_id_re = Regex::new(r"/song/(\d+)").unwrap();
                    let song_id: i64 = song_id_re
                        .captures(&song_url)
                        .and_then(|caps| caps[1].parse().ok()) // 尝试解析为数字
                        .unwrap_or(0); // 失败时默认0

                    // 第4列：歌手名
                    let artist = cols[3].text().collect::<String>().trim().to_string();

                    results.push(SongRank {
                        rank,
                        title,
                        artist,
                        song_id,
                        cover_url,
                        song_url,
                    });
                }
            }
        }

        results
    }

    /// 提取歌手列表
    ///
    /// 【知识点：相似但不相同的解析逻辑】
    /// 与extract_songs结构类似，但提取不同字段
    /// 这种重复是合理的，避免过度抽象导致代码复杂
    pub fn extract_singers(&self, doc: &Html) -> Vec<SingerRank> {
        let mut results = Vec::new();

        let table_selector = Selector::parse("table#myTable").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let col_selector = Selector::parse("td").unwrap();

        if let Some(table) = doc.select(&table_selector).next() {
            if let Some(tbody) = table.select(&tbody_selector).next() {
                for row in tbody.select(&row_selector) {
                    let cols: Vec<_> = row.select(&col_selector).collect();
                    if cols.len() < 4 {
                        continue;
                    }

                    let rank_text = cols[0].text().collect::<String>().trim().to_string();
                    let rank: i32 = rank_text.parse().unwrap_or(0);

                    // 第2列：头像和链接
                    let avatar_link = cols[1].select(&Selector::parse("a").unwrap()).next();
                    let img = cols[1].select(&Selector::parse("img").unwrap()).next();

                    let avatar_url = img
                        .map(|i| i.value().attr("src").unwrap_or("").to_string())
                        .unwrap_or_default();
                    let songs_url = avatar_link
                        .map(|l| l.value().attr("href").unwrap_or("").to_string())
                        .unwrap_or_default();

                    // 第3列：歌手名称
                    let name = cols[2].text().collect::<String>().trim().to_string();

                    results.push(SingerRank {
                        rank,
                        name,
                        avatar_url,
                        songs_url,
                    });
                }
            }
        }

        results
    }

    /// 提取分页信息
    ///
    /// 【知识点：复杂的HTML解析】
    /// 从分页导航栏提取：首页、上一页、下一页、尾页的URL
    /// 以及当前状态和总页数
    pub fn extract_pagination(&self, doc: &Html, current_page: i32) -> Pagination {
        let nav_selector = Selector::parse("nav[aria-label='Page navigation']").unwrap();

        if let Some(nav) = doc.select(&nav_selector).next() {
            let item_selector = Selector::parse("li.page-item").unwrap();
            let link_selector = Selector::parse("a").unwrap();

            let mut first_url = None;
            let mut prev_url = None;
            let mut next_url = None;
            let mut last_url = None;
            let mut has_prev = false;
            let mut has_next = false;
            let mut total_pages = 1;

            // 遍历所有分页按钮
            for item in nav.select(&item_selector) {
                if let Some(link) = item.select(&link_selector).next() {
                    let text = link.text().collect::<String>().trim().to_string();
                    let href = link.value().attr("href").unwrap_or("").to_string();

                    // 检查是否禁用（通过class判断）
                    let is_disabled = item
                        .value()
                        .attr("class")
                        .map(|c| c.contains("disabled"))
                        .unwrap_or(false);

                    // 根据按钮文本识别类型
                    if text.contains("首页") {
                        first_url = Some(href);
                    } else if text.contains("上一页") {
                        prev_url = Some(href);
                        has_prev = !is_disabled;
                    } else if text.contains("下一页") {
                        next_url = Some(href);
                        has_next = !is_disabled;
                    } else if text.contains("尾页") {
                        last_url = Some(href.clone());
                        // 从URL提取总页数
                        let re = Regex::new(r"page=(\d+)").unwrap();
                        if let Some(caps) = re.captures(&href) {
                            total_pages = caps[1].parse().unwrap_or(1);
                        }
                        // 歌手榜的URL格式不同
                        let singer_re = Regex::new(r"/singer/(\d+)").unwrap();
                        if let Some(caps) = singer_re.captures(&href) {
                            total_pages = caps[1].parse().unwrap_or(1);
                        }
                    }
                }
            }

            let (_, total_songs) = self.extract_ranking_name(doc);

            Pagination {
                current_page,
                total_pages,
                total_songs,
                has_prev,
                has_next,
                first_page_url: first_url,
                prev_page_url: prev_url,
                next_page_url: next_url,
                last_page_url: last_url,
            }
        } else {
            // 没有分页导航，返回默认值
            Pagination {
                current_page,
                total_pages: 1,
                total_songs: 0,
                has_prev: false,
                has_next: false,
                first_page_url: None,
                prev_page_url: None,
                next_page_url: None,
                last_page_url: None,
            }
        }
    }

    /// 提取所有数据
    ///
    /// 【知识点：动态类型识别】
    /// 根据榜单名称判断是歌手榜还是歌曲榜
    /// 返回不同填充的数据结构
    pub fn extract_all(&self, doc: &Html, current_page: i32) -> RankingPageData {
        let (ranking_name, _) = self.extract_ranking_name(doc);

        // 简单启发式：名称包含"歌手"认为是歌手榜
        if ranking_name.contains("歌手") {
            RankingPageData {
                ranking_name,
                songs: Vec::new(),
                singers: self.extract_singers(doc),
                pagination: self.extract_pagination(doc, current_page),
            }
        } else {
            RankingPageData {
                ranking_name,
                songs: self.extract_songs(doc),
                singers: Vec::new(),
                pagination: self.extract_pagination(doc, current_page),
            }
        }
    }

    /// 保存到JSON
    pub fn save_to_json(&self, data: &RankingPageData, filepath: &str) -> Result<()> {
        let path = Path::new(filepath);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("创建输出目录失败")?;
        }

        let content = serde_json::to_string_pretty(data).context("序列化数据失败")?;
        std::fs::write(path, content).context("写入JSON文件失败")?;
        Ok(())
    }
}

// 【扩展知识：网页解析的鲁棒性】
//
// 1. 防御性编程：
//    - 每个.select()后都检查.is_some()
//    - 使用unwrap_or_default()提供默认值
//    - 数据不完整时跳过而非崩溃
//
// 2. 页面结构变化处理：
//    - 使用更通用的选择器（如ID选择器优于嵌套选择器）
//    - 记录解析失败的元素，便于调试
//    - 考虑使用多个备选选择器
//
// 3. 性能优化：
//    - 预编译所有选择器（Selector::parse较昂贵）
//    - 避免不必要的字符串拷贝
//    - 使用迭代器而非collect后再处理
