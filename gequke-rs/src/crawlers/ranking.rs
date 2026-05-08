use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongRank {
    pub rank: i32,
    pub title: String,
    pub artist: String,
    pub song_id: i64,
    pub cover_url: String,
    pub song_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingerRank {
    pub rank: i32,
    pub name: String,
    pub avatar_url: String,
    pub songs_url: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingPageData {
    pub ranking_name: String,
    pub songs: Vec<SongRank>,
    pub singers: Vec<SingerRank>,
    pub pagination: Pagination,
}

pub struct RankingCrawler {
    cookie: Option<String>,
    user_agent: String,
    timeout: f64,
    client: reqwest::Client,
    ranking_types: HashMap<&'static str, &'static str>,
}

impl RankingCrawler {
    pub fn new(cookie: Option<String>, user_agent: Option<String>, timeout: Option<f64>) -> Self {
        let ua = user_agent.unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string());
        let t = timeout.unwrap_or(30.0);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("User-Agent", reqwest::header::HeaderValue::from_str(&ua).unwrap());
        headers.insert("Accept", reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"));
        headers.insert("Accept-Language", reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(t))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

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

    pub fn get_ranking_types(&self) -> &HashMap<&'static str, &'static str> {
        &self.ranking_types
    }

    fn get_cookies(&self) -> Option<String> {
        self.cookie.clone()
    }

    pub async fn get_ranking_page(&self, ranking_type: &str, page: i32) -> Result<Html> {
        if !self.ranking_types.contains_key(ranking_type) {
            return Err(anyhow::anyhow!("无效的榜单类型: {}", ranking_type));
        }

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

    pub fn get_ranking_page_from_file(filepath: &str) -> Result<Html> {
        let content = std::fs::read_to_string(Path::new(filepath))
            .context("读取本地HTML文件失败")?;
        Ok(Html::parse_document(&content))
    }

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

    pub fn extract_songs(&self, doc: &Html) -> Vec<SongRank> {
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

                    let img = cols[1].select(&Selector::parse("img").unwrap()).next();
                    let cover_url = img.map(|i| i.value().attr("src").unwrap_or("").to_string()).unwrap_or_default();

                    let song_link = cols[2].select(&Selector::parse("a").unwrap()).next();
                    let title = song_link.map(|l| l.text().collect::<String>().trim().to_string()).unwrap_or_default();
                    let song_url = song_link.map(|l| l.value().attr("href").unwrap_or("").to_string()).unwrap_or_default();

                    let song_id_re = Regex::new(r"/song/(\d+)").unwrap();
                    let song_id: i64 = song_id_re.captures(&song_url)
                        .and_then(|caps| caps[1].parse().ok())
                        .unwrap_or(0);

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

                    let avatar_link = cols[1].select(&Selector::parse("a").unwrap()).next();
                    let img = cols[1].select(&Selector::parse("img").unwrap()).next();

                    let avatar_url = img.map(|i| i.value().attr("src").unwrap_or("").to_string()).unwrap_or_default();
                    let songs_url = avatar_link.map(|l| l.value().attr("href").unwrap_or("").to_string()).unwrap_or_default();

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

            for item in nav.select(&item_selector) {
                if let Some(link) = item.select(&link_selector).next() {
                    let text = link.text().collect::<String>().trim().to_string();
                    let href = link.value().attr("href").unwrap_or("").to_string();
                    let is_disabled = item.value().attr("class")
                        .map(|c| c.contains("disabled"))
                        .unwrap_or(false);

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
                        let re = Regex::new(r"page=(\d+)").unwrap();
                        if let Some(caps) = re.captures(&href) {
                            total_pages = caps[1].parse().unwrap_or(1);
                        }
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

    pub fn extract_all(&self, doc: &Html, current_page: i32) -> RankingPageData {
        let (ranking_name, _) = self.extract_ranking_name(doc);

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