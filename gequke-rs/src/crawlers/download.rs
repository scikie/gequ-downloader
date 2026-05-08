use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongInfo {
    pub mp3_id: Option<String>,
    pub play_id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub cover: Option<String>,
    pub lrc: Option<String>,
    pub extra_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub success: bool,
    pub mp3_path: Option<String>,
    pub lrc_path: Option<String>,
    pub error: Option<String>,
    pub song_info: Option<DownloadSongInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSongInfo {
    pub song_id: i64,
    pub title: String,
    pub artist: String,
    pub cover_url: Option<String>,
    pub play_id: Option<String>,
}

pub struct DownloadCrawler {
    output_dir: PathBuf,
    cookie: Option<String>,
    user_agent: String,
    timeout: f64,
    client: reqwest::Client,
}

impl DownloadCrawler {
    pub fn new(output_dir: &str, cookie: Option<String>, user_agent: Option<String>, timeout: Option<f64>) -> Self {
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

        let output = PathBuf::from(output_dir);
        std::fs::create_dir_all(&output).ok();

        Self {
            output_dir: output,
            cookie,
            user_agent: ua,
            timeout: t,
            client,
        }
    }

    fn get_cookies(&self) -> Option<String> {
        self.cookie.clone()
    }

    pub async fn get_song_page(&self, song_id: i64) -> Result<Html> {
        let url = format!("https://www.gequke.com/song/{}", song_id);
        
        let mut request = self.client.get(&url);
        
        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("获取歌曲页面失败")?;
        let text = resp.text().await.context("读取歌曲页面内容失败")?;
        Ok(Html::parse_document(&text))
    }

    pub fn extract_song_info(&self, doc: &Html) -> SongInfo {
        let mut info = SongInfo {
            mp3_id: None,
            play_id: None,
            title: None,
            author: None,
            cover: None,
            lrc: None,
            extra_url: None,
        };

        let script_selector = Selector::parse("script").unwrap();
        for script in doc.select(&script_selector) {
            if let Some(script_text) = script.text().next() {
                if script_text.contains("window.mp3_id") {
                    let patterns = [
                        ("mp3_id", r"window\.mp3_id\s*=\s*'([^']+)'"),
                        ("play_id", r"window\.play_id\s*=\s*'([^']+)'"),
                        ("mp3_title", r"window\.mp3_title\s*=\s*'([^']+)'"),
                        ("mp3_author", r"window\.mp3_author\s*=\s*'([^']+)'"),
                        ("mp3_cover", r"window\.mp3_cover\s*=\s*'([^']+)'"),
                        ("mp3_extra_url", r"window\.mp3_extra_url\s*=\s*'([^']+)'"),
                    ];

                    for (key, pattern) in patterns {
                        let re = Regex::new(pattern).unwrap();
                        if let Some(caps) = re.captures(script_text) {
                            let value = caps[1].to_string();
                            match key {
                                "mp3_id" => info.mp3_id = Some(value),
                                "play_id" => info.play_id = Some(value),
                                "mp3_title" => info.title = Some(value),
                                "mp3_author" => info.author = Some(value),
                                "mp3_cover" => info.cover = Some(value),
                                "mp3_extra_url" => info.extra_url = Some(value),
                                _ => {}
                            }
                        }
                    }
                    break;
                }
            }
        }

        let lrc_selector = Selector::parse("div#content-lrc2").unwrap();
        if let Some(lrc_div) = doc.select(&lrc_selector).next() {
            let lrc_html = lrc_div.inner_html();
            let lrc_text = lrc_html.replace("<br/>", "\n").replace("<br />", "\n");
            info.lrc = Some(lrc_text);
        }

        info
    }

    pub async fn get_mp3_url(&self, play_id: &str, song_id: i64) -> Result<Option<String>> {
        let api_url = "https://www.gequke.com/api/music";

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json, text/javascript, */*; q=0.01"));
        headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"));
        headers.insert("X-Requested-With", reqwest::header::HeaderValue::from_static("XMLHttpRequest"));
        headers.insert("X-Custom-Header", reqwest::header::HeaderValue::from_static("SecretKey"));
        headers.insert("Origin", reqwest::header::HeaderValue::from_static("https://www.gequke.com"));
        headers.insert("Referer", reqwest::header::HeaderValue::from_str(&format!("https://www.gequke.com/song/{}", song_id)).unwrap());

        let body = format!("id={}&type=0", play_id);

        let mut request = self.client
            .post(api_url)
            .headers(headers)
            .body(body);

        if let Some(cookie) = self.get_cookies() {
            request = request.header("Cookie", cookie);
        }

        let resp = request.send().await.context("请求API失败")?;

        if resp.status() != 200 {
            return Ok(None);
        }

        let json: serde_json::Value = resp.json().await.context("解析API响应失败")?;
        if let Some(code) = json.get("code") {
            if code.as_i64() == Some(200) {
                if let Some(url) = json.get("data").and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
                    return Ok(Some(url.to_string()));
                }
            }
        }

        Ok(None)
    }

    pub async fn download_file(&self, url: &str, filepath: &Path) -> Result<String> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Referer", reqwest::header::HeaderValue::from_static("https://www.gequke.com/"));
        headers.insert("User-Agent", reqwest::header::HeaderValue::from_str(&self.user_agent).unwrap());

        let resp = self.client
            .get(url)
            .headers(headers)
            .send()
            .await
            .context("下载文件失败")?;

        let bytes = resp.bytes().await.context("读取文件内容失败")?;

        let mut actual_path = filepath.to_path_buf();
        let url_lower = url.to_lowercase();
        if url_lower.contains(".aac") && filepath.extension().map(|e| e != "aac").unwrap_or(false) {
            actual_path = filepath.with_extension("aac");
        } else if url_lower.contains(".flac") && filepath.extension().map(|e| e != "flac").unwrap_or(false) {
            actual_path = filepath.with_extension("flac");
        } else if url_lower.contains(".m4a") && filepath.extension().map(|e| e != "m4a").unwrap_or(false) {
            actual_path = filepath.with_extension("m4a");
        }

        let mut file = std::fs::File::create(&actual_path).context("创建文件失败")?;
        file.write_all(&bytes).context("写入文件失败")?;

        Ok(actual_path.to_string_lossy().to_string())
    }

    pub async fn download_cover(&self, cover_url: &str) -> Result<Vec<u8>> {
        let resp = self.client
            .get(cover_url)
            .send()
            .await
            .context("下载封面失败")?;

        let bytes = resp.bytes().await.context("读取封面内容失败")?;
        Ok(bytes.to_vec())
    }

    pub async fn download_song(&self, song_id: i64, embed_cover: bool) -> DownloadResult {
        let mut result = DownloadResult {
            success: false,
            mp3_path: None,
            lrc_path: None,
            error: None,
            song_info: None,
        };

        let doc = match self.get_song_page(song_id).await {
            Ok(d) => d,
            Err(e) => {
                result.error = Some(format!("获取歌曲页面失败: {}", e));
                return result;
            }
        };

        let info = self.extract_song_info(&doc);

        if info.play_id.is_none() {
            result.error = Some("未找到 play_id，可能需要登录".to_string());
            return result;
        }

        let title = info.title.clone().unwrap_or_else(|| "Unknown".to_string());
        let author = info.author.clone().unwrap_or_else(|| "Unknown".to_string());

        result.song_info = Some(DownloadSongInfo {
            song_id,
            title: title.clone(),
            artist: author.clone(),
            cover_url: info.cover.clone(),
            play_id: info.play_id.clone(),
        });

        println!("\x1b[36m歌曲: {} - {}\x1b[0m", title, author);

        let mp3_filename = format!("{}-{}.mp3", title, author);
        let mp3_filepath = self.output_dir.join(sanitize_filename(&mp3_filename));

        let lrc_filename = format!("{}-{}.lrc", title, author);
        let lrc_filepath = self.output_dir.join(sanitize_filename(&lrc_filename));

        let mut _cover_data: Option<Vec<u8>> = None;
        if embed_cover {
            if let Some(ref cover_url) = info.cover {
                match self.download_cover(cover_url).await {
                    Ok(data) => _cover_data = Some(data),
                    Err(e) => println!("\x1b[33m下载封面失败: {}\x1b[0m", e),
                }
            }
        }

        let mp3_url = match self.get_mp3_url(info.play_id.as_ref().unwrap(), song_id).await {
            Ok(Some(url)) => url,
            Ok(None) => {
                let mut error_msg = "API 未返回音频链接".to_string();
                if let Some(extra) = info.extra_url {
                    error_msg.push_str(&format!("，备用链接: {}", extra));
                }
                result.error = Some(error_msg);
                return result;
            }
            Err(e) => {
                result.error = Some(format!("获取MP3链接失败: {}", e));
                return result;
            }
        };

        match self.download_file(&mp3_url, &mp3_filepath).await {
            Ok(actual_path) => {
                result.mp3_path = Some(actual_path);
                result.success = true;
            }
            Err(e) => {
                result.error = Some(format!("下载 MP3 失败: {}", e));
                return result;
            }
        }

        if let Some(lrc) = info.lrc {
            if !lrc.is_empty() {
                match std::fs::write(&lrc_filepath, lrc) {
                    Ok(_) => result.lrc_path = Some(lrc_filepath.to_string_lossy().to_string()),
                    Err(e) => println!("\x1b[33m保存歌词失败: {}\x1b[0m", e),
                }
            }
        }

        result
    }
}

fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut result = String::new();
    for c in filename.chars() {
        if !invalid_chars.contains(&c) {
            result.push(c);
        }
    }
    result
}