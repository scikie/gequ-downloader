use anyhow::{Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongSearchResult {
    pub position: i32,
    pub song_id: i64,
    pub title: String,
    pub artist: String,
    pub song_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub keyword: String,
    pub total_count: i32,
    pub songs: Vec<SongSearchResult>,
}

pub struct SearchCrawler {
    cookie: Option<String>,
    user_agent: String,
    timeout: f64,
    client: reqwest::Client,
}

impl SearchCrawler {
    pub fn new(cookie: Option<String>, user_agent: Option<String>, timeout: Option<f64>) -> Self {
        let ua = user_agent.unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string());
        let t = timeout.unwrap_or(30.0);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "User-Agent",
            reqwest::header::HeaderValue::from_str(&ua).unwrap(),
        );
        headers.insert("Accept", reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"));
        headers.insert(
            "Accept-Language",
            reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(t))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();

        Self {
            cookie,
            user_agent: ua,
            timeout: t,
            client,
        }
    }

    fn get_cookies(&self) -> Option<String> {
        self.cookie.clone()
    }

    pub async fn search(&self, keyword: &str) -> Result<Html> {
        let url = format!("https://www.gequke.com/ss/{}", keyword);

        let mut request = self.client.get(&url);

        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("搜索请求失败")?;
        let text = resp.text().await.context("读取搜索结果失败")?;
        Ok(Html::parse_document(&text))
    }

    pub fn search_from_file(filepath: &str) -> Result<Html> {
        let content =
            std::fs::read_to_string(Path::new(filepath)).context("读取本地HTML文件失败")?;
        Ok(Html::parse_document(&content))
    }

    pub fn extract_keyword(&self, doc: &Html) -> String {
        let input_selector = Selector::parse("input#s-input-line").unwrap();
        if let Some(input) = doc.select(&input_selector).next() {
            if let Some(value) = input.value().attr("value") {
                return value.to_string();
            }
        }

        let h1_selector = Selector::parse("h1.navbar-h1").unwrap();
        if let Some(h1) = doc.select(&h1_selector).next() {
            return h1.text().collect::<String>().trim().to_string();
        }

        String::new()
    }

    pub fn extract_total_count(&self, doc: &Html) -> i32 {
        let div_selector = Selector::parse("div.quote-warning").unwrap();
        if let Some(div) = doc.select(&div_selector).next() {
            let text = div.text().collect::<String>();
            let re = Regex::new(r"共找到\s*(\d+)\s*条").unwrap();
            if let Some(caps) = re.captures(&text) {
                return caps[1].parse().unwrap_or(0);
            }
        }
        0
    }

    pub fn extract_songs(&self, doc: &Html) -> Vec<SongSearchResult> {
        let mut results = Vec::new();

        let table_selector = Selector::parse("table#myTables").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let col_selector = Selector::parse("td").unwrap();

        if let Some(table) = doc.select(&table_selector).next() {
            if let Some(tbody) = table.select(&tbody_selector).next() {
                for row in tbody.select(&row_selector) {
                    let cols: Vec<_> = row.select(&col_selector).collect();
                    if cols.len() < 3 {
                        continue;
                    }

                    let position_text = cols[0].text().collect::<String>().trim().to_string();
                    let position: i32 = position_text.parse().unwrap_or(0);

                    let song_link = cols[1].select(&Selector::parse("a").unwrap()).next();
                    let title = song_link
                        .map(|l| l.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();
                    let song_url = song_link
                        .map(|l| l.value().attr("href").unwrap_or("").to_string())
                        .unwrap_or_default();

                    let song_id_re = Regex::new(r"/song/(\d+)").unwrap();
                    let song_id: i64 = song_id_re
                        .captures(&song_url)
                        .and_then(|caps| caps[1].parse().ok())
                        .unwrap_or(0);

                    let artist = cols[2].text().collect::<String>().trim().to_string();

                    results.push(SongSearchResult {
                        position,
                        song_id,
                        title,
                        artist,
                        song_url,
                    });
                }
            }
        }

        results
    }

    pub fn extract_all(&self, doc: &Html) -> SearchResult {
        SearchResult {
            keyword: self.extract_keyword(doc),
            total_count: self.extract_total_count(doc),
            songs: self.extract_songs(doc),
        }
    }

    pub fn save_to_json(&self, data: &SearchResult, filepath: &str) -> Result<()> {
        let path = Path::new(filepath);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("创建输出目录失败")?;
        }

        let content = serde_json::to_string_pretty(data).context("序列化数据失败")?;
        std::fs::write(path, content).context("写入JSON文件失败")?;
        Ok(())
    }
}
