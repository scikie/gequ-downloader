use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKeywordItem {
    pub keyword: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedKeyword {
    pub rank: i32,
    pub keyword: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSinger {
    pub rank: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomepageData {
    pub latest_searches: Vec<SearchKeywordItem>,
    pub hot_keywords: Vec<RankedKeyword>,
    pub hot_singers: Vec<HotSinger>,
}

pub struct HomepageCrawler {
    cookie: Option<String>,
    user_agent: String,
    timeout: f64,
    client: reqwest::Client,
}

impl HomepageCrawler {
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

    pub async fn get_homepage(&self) -> Result<Html> {
        let mut request = self.client.get("https://www.gequke.com/");
        
        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("请求主页失败")?;
        let text = resp.text().await.context("读取主页内容失败")?;
        Ok(Html::parse_document(&text))
    }

    pub fn get_homepage_from_file(filepath: &str) -> Result<Html> {
        let content = std::fs::read_to_string(Path::new(filepath))
            .context("读取本地HTML文件失败")?;
        Ok(Html::parse_document(&content))
    }

    pub fn extract_latest_searches(&self, doc: &Html) -> Vec<SearchKeywordItem> {
        let mut results = Vec::new();
        
        let selector = Selector::parse("div.ilingku_singerlist a").unwrap();
        for element in doc.select(&selector) {
            let keyword = element.text().collect::<String>().trim().to_string();
            let url = element.value().attr("href").unwrap_or("").to_string();
            results.push(SearchKeywordItem { keyword, url });
        }

        results
    }

    pub fn extract_hot_keywords(&self, doc: &Html) -> Vec<RankedKeyword> {
        let mut results = Vec::new();

        let card_selector = Selector::parse("div.card").unwrap();
        let card_body_selector = Selector::parse("div.card-body").unwrap();
        let table_selector = Selector::parse("table.table").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();

        'outer: for card in doc.select(&card_selector) {
            if let Some(card_body) = card.select(&card_body_selector).next() {
                for h6 in card_body.select(&Selector::parse("h6").unwrap()) {
                    let text = h6.text().collect::<String>();
                    if text.contains("大家都在搜") {
                        if let Some(table) = card_body.select(&table_selector).next() {
                            if let Some(tbody) = table.select(&tbody_selector).next() {
                                for row in tbody.select(&row_selector) {
                                    let badge_sel = Selector::parse("span.badge").unwrap();
                                    let link_sel = Selector::parse("a").unwrap();
                                    
                                    let rank_badge = row.select(&badge_sel).next();
                                    let keyword_link = row.select(&link_sel).next();

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
                        break 'outer;
                    }
                }
            }
        }

        results
    }

    pub fn extract_hot_singers(&self, doc: &Html) -> Vec<HotSinger> {
        let mut results = Vec::new();

        let card_selector = Selector::parse("div.card").unwrap();
        let card_body_selector = Selector::parse("div.card-body").unwrap();
        let table_selector = Selector::parse("table.table").unwrap();
        let tbody_selector = Selector::parse("tbody").unwrap();
        let row_selector = Selector::parse("tr").unwrap();

        'outer: for card in doc.select(&card_selector) {
            if let Some(card_body) = card.select(&card_body_selector).next() {
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

    pub fn extract_all(&self, doc: &Html) -> HomepageData {
        HomepageData {
            latest_searches: self.extract_latest_searches(doc),
            hot_keywords: self.extract_hot_keywords(doc),
            hot_singers: self.extract_hot_singers(doc),
        }
    }

    pub fn save_to_json(&self, data: &HomepageData, filepath: &str) -> Result<()> {
        let path = Path::new(filepath);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("创建输出目录失败")?;
        }
        
        let content = serde_json::to_string_pretty(data).context("序列化数据失败")?;
        std::fs::write(path, content).context("写入JSON文件失败")?;
        Ok(())
    }
}