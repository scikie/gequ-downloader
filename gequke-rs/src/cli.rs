use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use tabled::{Table, Tabled, settings::Style};

use crate::config::GequConfig;
use crate::database::Database;
use crate::models::{get_ranking_types, get_page_types, Song, Singer, RankingItem, PageSnapshot, PageItem};
use crate::crawlers::{HomepageCrawler, RankingCrawler, SearchCrawler, DownloadCrawler};

#[derive(Parser)]
#[command(name = "gequ")]
#[command(about = "歌曲客网站爬虫工具", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
    Crawl(CrawlCommands),
    
    #[command(subcommand)]
    Download(DownloadCommands),
    
    #[command(subcommand)]
    Db(DbCommands),
    
    #[command(subcommand)]
    Stats(StatsCommands),
    
    Search {
        keyword: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
        #[arg(short, long)]
        file: Option<String>,
    },
    
    #[command(subcommand)]
    Config(ConfigCommands),
    
    Version,
}

#[derive(Subcommand)]
enum CrawlCommands {
    Homepage {
        #[arg(short, long)]
        file: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
    
    Ranking {
        ranking_type: String,
        #[arg(short, long, default_value = "1")]
        page: i32,
        #[arg(short = 's', long)]
        start_page: Option<i32>,
        #[arg(short = 'e', long)]
        end_page: Option<i32>,
        #[arg(short, long)]
        file: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
}

#[derive(Subcommand)]
enum DownloadCommands {
    Song {
        song_id: i64,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_cover: bool,
        #[arg(long)]
        no_db: bool,
    },
    
    Songs {
        song_ids: Vec<i64>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    Stats,
    
    Singer {
        name: Option<String>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
    
    Song {
        song_id: Option<i64>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
    
    Download {
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
    
    Ranking {
        ranking_type: Option<String>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
}

#[derive(Subcommand)]
enum StatsCommands {
    Song {
        song_id: i64,
        #[arg(long)]
        history: bool,
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
    },
    
    Singer {
        name: String,
        #[arg(long)]
        history: bool,
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
    },
    
    TopSongs {
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        last: Option<String>,
    },
    
    TopSingers {
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        last: Option<String>,
    },
    
    History {
        page_type: String,
        ranking_type: Option<String>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        last: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    Show,
    
    Set {
        key: String,
        value: String,
    },
    
    Get {
        key: String,
    },
    
    Reset,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Crawl(cmd) => handle_crawl(cmd).await?,
        Commands::Download(cmd) => handle_download(cmd).await?,
        Commands::Db(cmd) => handle_db(cmd)?,
        Commands::Stats(cmd) => handle_stats(cmd)?,
        Commands::Search { keyword, output, no_db, file } => handle_search(keyword, output, no_db, file).await?,
        Commands::Config(cmd) => handle_config(cmd)?,
        Commands::Version => println!("gequ version {}", env!("CARGO_PKG_VERSION")),
    }
    
    Ok(())
}

async fn handle_crawl(cmd: CrawlCommands) -> Result<()> {
    let config = GequConfig::load()?;
    
    match cmd {
        CrawlCommands::Homepage { file, output, no_db } => {
            let crawler = HomepageCrawler::new(
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            println!("{}", "正在爬取主页...".cyan());
            
            let doc = if let Some(filepath) = file {
                HomepageCrawler::get_homepage_from_file(&filepath)?
            } else {
                crawler.get_homepage().await?
            };
            
            let data = crawler.extract_all(&doc);
            
            println!("{}", "成功提取数据".green());
            
            #[derive(Tabled)]
            struct StatRow {
                #[tabled(rename = "类型")]
                r#type: &'static str,
                #[tabled(rename = "数量")]
                count: usize,
            }
            
            let rows = vec![
                StatRow { r#type: "最新搜索", count: data.latest_searches.len() },
                StatRow { r#type: "热门关键词", count: data.hot_keywords.len() },
                StatRow { r#type: "热门歌手", count: data.hot_singers.len() },
            ];
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
            
            if !data.latest_searches.is_empty() {
                println!("\n{}", "最新搜索:".bold());
                for item in data.latest_searches.iter().take(5) {
                    println!("  - {}", item.keyword);
                }
            }
            
            if !no_db {
                let db = Database::new(&config.db_path)?;
                
                let snapshot = PageSnapshot {
                    page_type: "homepage".to_string(),
                    ranking_type: None,
                    search_keyword: None,
                    page_number: 1,
                    url: Some("https://www.gequke.com/".to_string()),
                    title: Some("歌曲客主页".to_string()),
                };
                let snapshot_id = db.insert_page_snapshot(&snapshot)?;
                
                let mut page_items = Vec::new();
                for item in &data.hot_singers {
                    let singer = Singer {
                        name: item.name.clone(),
                        avatar_url: None,
                        songs_url: Some(item.url.clone()),
                    };
                    let singer_id = db.insert_singer(&singer)?;
                    
                    page_items.push(PageItem {
                        page_snapshot_id: snapshot_id,
                        item_type: "singer".to_string(),
                        item_id: Some(singer_id),
                        position: item.rank,
                        extra_data: Some(serde_json::json!({"url": item.url}).to_string()),
                    });
                }
                
                db.insert_page_items(&page_items)?;
                println!("\n{}", "已保存到数据库".green());
            }
            
            if let Some(output_path) = output {
                crawler.save_to_json(&data, &output_path)?;
                println!("\n{} {}", "已保存到:".green(), output_path);
            }
        }
        
        CrawlCommands::Ranking { ranking_type, page, start_page, end_page, file, output, no_db } => {
            let crawler = RankingCrawler::new(
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            let ranking_types = crawler.get_ranking_types();
            let ranking_name = ranking_types.get(ranking_type.as_str())
                .ok_or_else(|| anyhow::anyhow!("无效的榜单类型: {}", ranking_type))?;
            
            if let (Some(s), Some(e)) = (start_page, end_page) {
                println!("{}", format!("正在爬取 {} 第 {}-{} 页...", ranking_name, s, e).cyan());
                
                let mut all_data = Vec::new();
                for p in s..=e {
                    println!("{}", format!("正在爬取第 {} 页...", p).cyan());
                    let doc = crawler.get_ranking_page(&ranking_type, p).await?;
                    let data = crawler.extract_all(&doc, p);
                    all_data.push(data);
                }
                
                println!("{}", format!("成功爬取 {} 页", e - s + 1).green());
                
                if !no_db {
                    let db = Database::new(&config.db_path)?;
                    let mut song_count = 0;
                    let mut singer_count = 0;
                    let mut ranking_count = 0;
                    
                    for data in &all_data {
                        let snapshot = PageSnapshot {
                            page_type: "ranking".to_string(),
                            ranking_type: Some(ranking_type.clone()),
                            search_keyword: None,
                            page_number: data.pagination.current_page,
                            url: None,
                            title: Some(data.ranking_name.clone()),
                        };
                        let snapshot_id = db.insert_page_snapshot(&snapshot)?;
                        
                        let mut page_items = Vec::new();
                        
                        if !data.singers.is_empty() {
                            for singer in &data.singers {
                                let s = Singer {
                                    name: singer.name.clone(),
                                    avatar_url: Some(singer.avatar_url.clone()),
                                    songs_url: Some(singer.songs_url.clone()),
                                };
                                let singer_id = db.insert_singer(&s)?;
                                singer_count += 1;
                                
                                let r = RankingItem {
                                    ranking_type: ranking_type.clone(),
                                    rank: singer.rank,
                                    item_id: None,
                                    item_name: Some(singer.name.clone()),
                                    item_type: "singer".to_string(),
                                };
                                db.insert_ranking_item(&r)?;
                                ranking_count += 1;
                                
                                page_items.push(PageItem {
                                    page_snapshot_id: snapshot_id,
                                    item_type: "singer".to_string(),
                                    item_id: Some(singer_id),
                                    position: singer.rank,
                                    extra_data: Some(serde_json::json!({
                                        "avatar_url": singer.avatar_url,
                                        "songs_url": singer.songs_url
                                    }).to_string()),
                                });
                            }
                        } else {
                            for song in &data.songs {
                                let s = Song {
                                    song_id: song.song_id,
                                    title: song.title.clone(),
                                    artist: song.artist.clone(),
                                    cover_url: Some(song.cover_url.clone()),
                                    mp3_url: None,
                                    play_id: None,
                                    lrc: None,
                                    extra_url: None,
                                };
                                let song_db_id = db.insert_song(&s)?;
                                song_count += 1;
                                
                                let r = RankingItem {
                                    ranking_type: ranking_type.clone(),
                                    rank: song.rank,
                                    item_id: Some(song.song_id),
                                    item_name: None,
                                    item_type: "song".to_string(),
                                };
                                db.insert_ranking_item(&r)?;
                                ranking_count += 1;
                                
                                page_items.push(PageItem {
                                    page_snapshot_id: snapshot_id,
                                    item_type: "song".to_string(),
                                    item_id: Some(song_db_id),
                                    position: song.rank,
                                    extra_data: Some(serde_json::json!({
                                        "title": song.title,
                                        "artist": song.artist,
                                        "cover_url": song.cover_url
                                    }).to_string()),
                                });
                            }
                        }
                        
                        db.insert_page_items(&page_items)?;
                    }
                    
                    println!("\n{} {} 位歌手, {} 首歌曲, {} 条排行记录", 
                        "已保存到数据库:".green(), singer_count, song_count, ranking_count);
                }
            } else {
                println!("{}", format!("正在爬取 {}...", ranking_name).cyan());
                
                let doc = if let Some(filepath) = file {
                    RankingCrawler::get_ranking_page_from_file(&filepath)?
                } else {
                    crawler.get_ranking_page(&ranking_type, page).await?
                };
                
                let data = crawler.extract_all(&doc, page);
                println!("{}", "成功提取数据".green());
                
                #[derive(Tabled)]
                struct SongRow {
                    #[tabled(rename = "排名")]
                    rank: i32,
                    #[tabled(rename = "歌曲")]
                    title: String,
                    #[tabled(rename = "歌手")]
                    artist: String,
                }
                
                #[derive(Tabled)]
                struct SingerRow {
                    #[tabled(rename = "排名")]
                    rank: i32,
                    #[tabled(rename = "歌手")]
                    name: String,
                }
                
                if !data.singers.is_empty() {
                    let rows: Vec<_> = data.singers.iter().take(10).map(|s| SingerRow {
                        rank: s.rank,
                        name: s.name.clone(),
                    }).collect();
                    let table = Table::new(rows).with(Style::rounded()).to_string();
                    println!("\n{}", table);
                } else {
                    let rows: Vec<_> = data.songs.iter().take(10).map(|s| SongRow {
                        rank: s.rank,
                        title: s.title.clone(),
                        artist: s.artist.clone(),
                    }).collect();
                    let table = Table::new(rows).with(Style::rounded()).to_string();
                    println!("\n{}", table);
                }
                
                if !no_db {
                    let db = Database::new(&config.db_path)?;
                    
                    let snapshot = PageSnapshot {
                        page_type: "ranking".to_string(),
                        ranking_type: Some(ranking_type.clone()),
                        search_keyword: None,
                        page_number: page,
                        url: None,
                        title: Some(data.ranking_name.clone()),
                    };
                    let snapshot_id = db.insert_page_snapshot(&snapshot)?;
                    
                    let mut page_items = Vec::new();
                    
                    if !data.singers.is_empty() {
                        for singer in &data.singers {
                            let s = Singer {
                                name: singer.name.clone(),
                                avatar_url: Some(singer.avatar_url.clone()),
                                songs_url: Some(singer.songs_url.clone()),
                            };
                            let singer_id = db.insert_singer(&s)?;
                            
                            page_items.push(PageItem {
                                page_snapshot_id: snapshot_id,
                                item_type: "singer".to_string(),
                                item_id: Some(singer_id),
                                position: singer.rank,
                                extra_data: None,
                            });
                        }
                    } else {
                        for song in &data.songs {
                            let s = Song {
                                song_id: song.song_id,
                                title: song.title.clone(),
                                artist: song.artist.clone(),
                                cover_url: Some(song.cover_url.clone()),
                                mp3_url: None,
                                play_id: None,
                                lrc: None,
                                extra_url: None,
                            };
                            let song_db_id = db.insert_song(&s)?;
                            
                            page_items.push(PageItem {
                                page_snapshot_id: snapshot_id,
                                item_type: "song".to_string(),
                                item_id: Some(song_db_id),
                                position: song.rank,
                                extra_data: None,
                            });
                        }
                    }
                    
                    db.insert_page_items(&page_items)?;
                    println!("\n{}", "已保存到数据库".green());
                }
                
                if let Some(output_path) = output {
                    crawler.save_to_json(&data, &output_path)?;
                    println!("\n{} {}", "已保存到:".green(), output_path);
                }
            }
        }
    }
    
    Ok(())
}

async fn handle_download(cmd: DownloadCommands) -> Result<()> {
    let config = GequConfig::load()?;
    
    match cmd {
        DownloadCommands::Song { song_id, output, no_cover, no_db } => {
            let output_dir = output.unwrap_or_else(|| config.download_dir.clone());
            let crawler = DownloadCrawler::new(
                &output_dir,
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            println!("{}", format!("正在下载歌曲 {}...", song_id).cyan());
            
            let result = crawler.download_song(song_id, !no_cover).await;
            
            if result.success {
                println!("{}", "下载成功".green());
                if let Some(ref path) = result.mp3_path {
                    println!("{}: {}", "音频".green(), path);
                }
                if let Some(ref path) = result.lrc_path {
                    println!("{}: {}", "歌词".green(), path);
                }
                
                if !no_db {
                    let db = Database::new(&config.db_path)?;
                    
                    if let Some(ref song_info) = result.song_info {
                        let song = Song {
                            song_id: song_info.song_id,
                            title: song_info.title.clone(),
                            artist: song_info.artist.clone(),
                            cover_url: song_info.cover_url.clone(),
                            mp3_url: None,
                            play_id: song_info.play_id.clone(),
                            lrc: None,
                            extra_url: None,
                        };
                        db.insert_song(&song)?;
                    }
                    
                    use std::path::Path;
                    let mp3_path = result.mp3_path.as_ref().unwrap();
                    let file_size = if Path::new(mp3_path).exists() {
                        Some(Path::new(mp3_path).metadata()?.len() as i64)
                    } else {
                        None
                    };
                    
                    use crate::models::DownloadRecord;
                    let record = DownloadRecord {
                        song_id,
                        file_path: result.mp3_path.clone().unwrap(),
                        file_size,
                        downloaded_at: None,
                    };
                    let record_id = db.insert_download_record(&record)?;
                    
                    println!("{} (ID: {})", "已保存下载记录到数据库".green(), record_id);
                }
            } else {
                println!("{}", "下载失败".red());
                if let Some(ref error) = result.error {
                    println!("{}: {}", "原因".red(), error);
                }
            }
        }
        
        DownloadCommands::Songs { song_ids, output, no_db } => {
            let output_dir = output.unwrap_or_else(|| config.download_dir.clone());
            let crawler = DownloadCrawler::new(
                &output_dir,
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            let mut success_count = 0;
            let mut fail_count = 0;
            
            for song_id in song_ids {
                let result = crawler.download_song(song_id, true).await;
                if result.success {
                    success_count += 1;
                    println!("{} {}", "✓".green(), song_id);
                    
                    if !no_db {
                        let db = Database::new(&config.db_path)?;
                        
                        if let Some(ref song_info) = result.song_info {
                            let song = Song {
                                song_id: song_info.song_id,
                                title: song_info.title.clone(),
                                artist: song_info.artist.clone(),
                                cover_url: song_info.cover_url.clone(),
                                mp3_url: None,
                                play_id: song_info.play_id.clone(),
                                lrc: None,
                                extra_url: None,
                            };
                            db.insert_song(&song)?;
                        }
                        
                        use std::path::Path;
                        let mp3_path = Path::new(result.mp3_path.as_ref().unwrap());
                        let file_size = if mp3_path.exists() {
                            Some(mp3_path.metadata()?.len() as i64)
                        } else {
                            None
                        };
                        
                        use crate::models::DownloadRecord;
                        let record = DownloadRecord {
                            song_id,
                            file_path: result.mp3_path.clone().unwrap(),
                            file_size,
                            downloaded_at: None,
                        };
                        db.insert_download_record(&record)?;
                    }
                } else {
                    fail_count += 1;
                    println!("{} {}", "✗".red(), song_id);
                }
            }
            
            println!("\n{}: {} 成功, {} 失败", "完成".bold(), success_count, fail_count);
        }
    }
    
    Ok(())
}

fn handle_db(cmd: DbCommands) -> Result<()> {
    let config = GequConfig::load()?;
    let db = Database::new(&config.db_path)?;
    
    match cmd {
        DbCommands::Stats => {
            let stats = db.get_stats()?;
            
            #[derive(Tabled)]
            struct StatRow {
                #[tabled(rename = "类型")]
                r#type: &'static str,
                #[tabled(rename = "数量")]
                count: i64,
            }
            
            let rows = vec![
                StatRow { r#type: "歌手数", count: stats["total_singers"].as_i64().unwrap_or(0) },
                StatRow { r#type: "歌曲数", count: stats["total_songs"].as_i64().unwrap_or(0) },
                StatRow { r#type: "排行记录数", count: stats["total_rankings"].as_i64().unwrap_or(0) },
                StatRow { r#type: "搜索关键词数", count: stats["total_keywords"].as_i64().unwrap_or(0) },
                StatRow { r#type: "下载记录数", count: stats["total_downloads"].as_i64().unwrap_or(0) },
                StatRow { r#type: "页面快照数", count: stats["total_page_snapshots"].as_i64().unwrap_or(0) },
                StatRow { r#type: "页面条目数", count: stats["total_page_items"].as_i64().unwrap_or(0) },
            ];
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        
        DbCommands::Singer { name, number } => {
            if let Some(singer_name) = name {
                if let Some(singer) = db.get_singer_by_name(&singer_name)? {
                    println!("{}", format!("歌手: {}", singer_name).bold());
                    println!("ID: {}", singer["id"]);
                    println!("名称: {}", singer["name"]);
                    println!("头像: {}", singer["avatar_url"].as_str().unwrap_or("-"));
                    println!("歌曲页: {}", singer["songs_url"].as_str().unwrap_or("-"));
                    println!("创建时间: {}", singer["created_at"].as_str().unwrap_or("-"));
                    
                    let stats = db.get_singer_appearance_stats(&singer_name)?;
                    println!("\n{}", "出现统计:".bold());
                    println!("  总次数: {}", stats["total_count"]);
                    println!("  主页: {}", stats["homepage_count"]);
                    println!("  排行榜: {}", stats["ranking_count"]);
                } else {
                    println!("{}", format!("未找到歌手: {}", singer_name).red());
                }
            } else {
                let singers = db.get_all_singers(number)?;
                
                #[derive(Tabled)]
                struct SingerRow {
                    #[tabled(rename = "ID")]
                    id: i64,
                    #[tabled(rename = "歌手")]
                    name: String,
                    #[tabled(rename = "创建时间")]
                    created_at: String,
                }
                
                let rows: Vec<_> = singers.iter().map(|s| SingerRow {
                    id: s["id"].as_i64().unwrap_or(0),
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    created_at: s["created_at"].as_str().unwrap_or("-").to_string(),
                }).collect();
                
                let table = Table::new(rows).with(Style::rounded()).to_string();
                println!("{}", table);
            }
        }
        
        DbCommands::Song { song_id, number } => {
            if let Some(id) = song_id {
                if let Some(song) = db.get_song_by_id(id)? {
                    println!("{}", format!("歌曲: {}", id).bold());
                    println!("ID: {}", song["id"]);
                    println!("歌曲ID: {}", song["song_id"]);
                    println!("标题: {}", song["title"]);
                    println!("歌手: {}", song["artist"]);
                    println!("封面: {}", song["cover_url"].as_str().unwrap_or("-"));
                    println!("创建时间: {}", song["created_at"].as_str().unwrap_or("-"));
                    
                    let stats = db.get_song_appearance_stats(id)?;
                    println!("\n{}", "出现统计:".bold());
                    println!("  总次数: {}", stats["total_count"]);
                    println!("  主页: {}", stats["homepage_count"]);
                    println!("  排行榜: {}", stats["ranking_count"]);
                } else {
                    println!("{}", format!("未找到歌曲: {}", id).red());
                }
            } else {
                let songs = db.get_all_songs(number)?;
                
                #[derive(Tabled)]
                struct SongRow {
                    #[tabled(rename = "歌曲ID")]
                    song_id: i64,
                    #[tabled(rename = "标题")]
                    title: String,
                    #[tabled(rename = "歌手")]
                    artist: String,
                    #[tabled(rename = "创建时间")]
                    created_at: String,
                }
                
                let rows: Vec<_> = songs.iter().map(|s| SongRow {
                    song_id: s["song_id"].as_i64().unwrap_or(0),
                    title: s["title"].as_str().unwrap_or("").to_string(),
                    artist: s["artist"].as_str().unwrap_or("").to_string(),
                    created_at: s["created_at"].as_str().unwrap_or("-").to_string(),
                }).collect();
                
                let table = Table::new(rows).with(Style::rounded()).to_string();
                println!("{}", table);
            }
        }
        
        DbCommands::Download { number } => {
            let downloads = db.get_all_downloads(number)?;
            
            #[derive(Tabled)]
            struct DownloadRow {
                #[tabled(rename = "ID")]
                id: i64,
                #[tabled(rename = "歌曲ID")]
                song_id: i64,
                #[tabled(rename = "标题")]
                title: String,
                #[tabled(rename = "歌手")]
                artist: String,
                #[tabled(rename = "文件大小")]
                size: String,
                #[tabled(rename = "下载时间")]
                time: String,
            }
            
            let rows: Vec<_> = downloads.iter().map(|d| DownloadRow {
                id: d["id"].as_i64().unwrap_or(0),
                song_id: d["song_id"].as_i64().unwrap_or(0),
                title: d["title"].as_str().unwrap_or("-").to_string(),
                artist: d["artist"].as_str().unwrap_or("-").to_string(),
                size: d["file_size"].as_i64().map(|s| format!("{}KB", s / 1024)).unwrap_or("-".to_string()),
                time: d["downloaded_at"].as_str().unwrap_or("-").to_string(),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        
        DbCommands::Ranking { ranking_type, number } => {
            let rankings = db.get_all_rankings(ranking_type.as_deref(), number)?;
            
            #[derive(Tabled)]
            struct RankingRow {
                #[tabled(rename = "排名")]
                rank: i32,
                #[tabled(rename = "类型")]
                ranking_type: String,
                #[tabled(rename = "标题/歌手")]
                item_name: String,
                #[tabled(rename = "歌手")]
                artist: String,
                #[tabled(rename = "抓取时间")]
                time: String,
            }
            
            let ranking_types = get_ranking_types();
            let rows: Vec<_> = rankings.iter().map(|r| RankingRow {
                rank: r["rank"].as_i64().unwrap_or(0) as i32,
                ranking_type: ranking_types.get(r["ranking_type"].as_str().unwrap_or(""))
                    .map(|s| s.to_string()).unwrap_or_else(|| r["ranking_type"].as_str().unwrap_or("").to_string()),
                item_name: r["singer_name"].as_str().map(|s| s.to_string())
                    .unwrap_or_else(|| r["title"].as_str().unwrap_or("-").to_string()),
                artist: r["artist"].as_str().unwrap_or("-").to_string(),
                time: r["crawled_at"].as_str().unwrap_or("-").to_string(),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
    }
    
    Ok(())
}

fn handle_stats(cmd: StatsCommands) -> Result<()> {
    let config = GequConfig::load()?;
    let db = Database::new(&config.db_path)?;
    
    match cmd {
        StatsCommands::Song { song_id, .. } => {
            if let Some(song) = db.get_song_by_id(song_id)? {
                println!("{}", format!("歌曲: {}", song_id).bold());
                println!("歌曲ID: {}", song["song_id"]);
                println!("标题: {}", song["title"]);
                println!("歌手: {}", song["artist"]);
                println!("封面: {}", song["cover_url"].as_str().unwrap_or("-"));
                println!("创建时间: {}", song["created_at"].as_str().unwrap_or("-"));
                
                let stats = db.get_song_appearance_stats(song_id)?;
                println!("\n{}", "出现统计:".bold());
                println!("  总次数: {}", stats["total_count"]);
                println!("  主页: {}", stats["homepage_count"]);
                println!("  排行榜: {}", stats["ranking_count"]);
            } else {
                println!("{}", format!("未找到歌曲: {}", song_id).red());
            }
        }
        
        StatsCommands::Singer { name, .. } => {
            if let Some(singer) = db.get_singer_by_name(&name)? {
                println!("{}", format!("歌手: {}", name).bold());
                println!("ID: {}", singer["id"]);
                println!("名称: {}", singer["name"]);
                println!("头像: {}", singer["avatar_url"].as_str().unwrap_or("-"));
                println!("歌曲页: {}", singer["songs_url"].as_str().unwrap_or("-"));
                println!("创建时间: {}", singer["created_at"].as_str().unwrap_or("-"));
                
                let stats = db.get_singer_appearance_stats(&name)?;
                println!("\n{}", "出现统计:".bold());
                println!("  总次数: {}", stats["total_count"]);
                println!("  主页: {}", stats["homepage_count"]);
                println!("  排行榜: {}", stats["ranking_count"]);
            } else {
                println!("{}", format!("未找到歌手: {}", name).red());
            }
        }
        
        StatsCommands::TopSongs { number, from, to, last } => {
            let (date_from, date_to) = parse_time_args(from, to, last)?;
            let songs = db.get_top_appearing_songs(number, date_from.as_deref(), date_to.as_deref())?;
            
            #[derive(Tabled)]
            struct TopSongRow {
                #[tabled(rename = "排名")]
                rank: usize,
                #[tabled(rename = "歌曲")]
                title: String,
                #[tabled(rename = "歌手")]
                artist: String,
                #[tabled(rename = "出现次数")]
                count: i64,
            }
            
            let rows: Vec<_> = songs.iter().enumerate().map(|(i, s)| TopSongRow {
                rank: i + 1,
                title: s["title"].as_str().unwrap_or("").to_string(),
                artist: s["artist"].as_str().unwrap_or("").to_string(),
                count: s["appearance_count"].as_i64().unwrap_or(0),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        
        StatsCommands::TopSingers { number, from, to, last } => {
            let (date_from, date_to) = parse_time_args(from, to, last)?;
            let singers = db.get_top_appearing_singers(number, date_from.as_deref(), date_to.as_deref())?;
            
            #[derive(Tabled)]
            struct TopSingerRow {
                #[tabled(rename = "排名")]
                rank: usize,
                #[tabled(rename = "歌手")]
                name: String,
                #[tabled(rename = "出现次数")]
                count: i64,
            }
            
            let rows: Vec<_> = singers.iter().enumerate().map(|(i, s)| TopSingerRow {
                rank: i + 1,
                name: s["name"].as_str().unwrap_or("").to_string(),
                count: s["appearance_count"].as_i64().unwrap_or(0),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        
        StatsCommands::History { page_type, number, from, to, last, .. } => {
            let page_types = get_page_types();
            if !page_types.contains_key(page_type.as_str()) {
                println!("{}", format!("无效的页面类型: {}", page_type).red());
                println!("支持类型: homepage, ranking, search");
                return Ok(());
            }
            
            let (date_from, date_to) = parse_time_args(from, to, last)?;
            let snapshots = db.get_page_snapshots(Some(&page_type), number, date_from.as_deref(), date_to.as_deref())?;
            
            if snapshots.is_empty() {
                println!("{}", "暂无数据".yellow());
                return Ok(());
            }
            
            #[derive(Tabled)]
            struct SnapshotRow {
                #[tabled(rename = "ID")]
                id: i64,
                #[tabled(rename = "时间")]
                time: String,
                #[tabled(rename = "标题")]
                title: String,
            }
            
            let rows: Vec<_> = snapshots.iter().map(|s| SnapshotRow {
                id: s["id"].as_i64().unwrap_or(0),
                time: s["crawled_at"].as_str().unwrap_or("-").to_string(),
                title: s["title"].as_str().unwrap_or("-").to_string(),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
    }
    
    Ok(())
}

fn parse_time_args(from: Option<String>, to: Option<String>, last: Option<String>) -> Result<(Option<String>, Option<String>)> {
    if let Some(last_str) = last {
        use regex::Regex;
        let re = Regex::new(r"^(\d+)([dDwWmM])$").unwrap();
        
        if let Some(caps) = re.captures(&last_str) {
            let num: i64 = caps[1].parse()?;
            let unit = caps[2].to_lowercase();
            
            use chrono::{Local, Duration};
            let now = Local::now();
            let delta = match unit.as_str() {
                "d" => Duration::days(num),
                "w" => Duration::weeks(num),
                "m" => Duration::days(num * 30),
                _ => return Err(anyhow::anyhow!("不支持的时间单位: {}", unit)),
            };
            
            let date_from = (now - delta).format("%Y-%m-%d %H:%M:%S").to_string();
            let date_to = now.format("%Y-%m-%d %H:%M:%S").to_string();
            
            return Ok((Some(date_from), Some(date_to)));
        } else {
            return Err(anyhow::anyhow!("无效的时间范围格式: {}", last_str));
        }
    }
    
    Ok((from, to))
}

async fn handle_search(keyword: String, output: Option<String>, no_db: bool, file: Option<String>) -> Result<()> {
    let config = GequConfig::load()?;
    let crawler = SearchCrawler::new(
        if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
        Some(config.user_agent.clone()),
        Some(config.timeout),
    );
    
    println!("{}", format!("正在搜索 '{}'...", keyword).cyan());
    
    let doc = if let Some(filepath) = file {
        SearchCrawler::search_from_file(&filepath)?
    } else {
        crawler.search(&keyword).await?
    };
    
    let data = crawler.extract_all(&doc);
    
    println!("{}", format!("找到 {} 条结果", data.total_count).green());
    
    if !data.songs.is_empty() {
        #[derive(Tabled)]
        struct SearchResultRow {
            #[tabled(rename = "序号")]
            position: i32,
            #[tabled(rename = "歌曲")]
            title: String,
            #[tabled(rename = "歌手")]
            artist: String,
            #[tabled(rename = "ID")]
            song_id: i64,
        }
        
        let rows: Vec<_> = data.songs.iter().take(20).map(|s| SearchResultRow {
            position: s.position,
            title: s.title.clone(),
            artist: s.artist.clone(),
            song_id: s.song_id,
        }).collect();
        
        let table = Table::new(rows).with(Style::rounded()).to_string();
        println!("\n{}", table);
    }
    
    if !no_db {
        let db = Database::new(&config.db_path)?;
        
        let snapshot = PageSnapshot {
            page_type: "search".to_string(),
            ranking_type: None,
            search_keyword: Some(data.keyword.clone()),
            page_number: 1,
            url: Some(format!("https://www.gequke.com/ss/{}", keyword)),
            title: Some(format!("搜索: {}", data.keyword)),
        };
        let snapshot_id = db.insert_page_snapshot(&snapshot)?;
        
        let mut page_items = Vec::new();
        let mut song_count = 0;
        
        for song in &data.songs {
            let s = Song {
                song_id: song.song_id,
                title: song.title.clone(),
                artist: song.artist.clone(),
                cover_url: None,
                mp3_url: None,
                play_id: None,
                lrc: None,
                extra_url: None,
            };
            let song_db_id = db.insert_song(&s)?;
            song_count += 1;
            
            page_items.push(PageItem {
                page_snapshot_id: snapshot_id,
                item_type: "song".to_string(),
                item_id: Some(song_db_id),
                position: song.position,
                extra_data: Some(serde_json::json!({"song_url": song.song_url}).to_string()),
            });
        }
        
        db.insert_page_items(&page_items)?;
        println!("\n{} {} 首歌曲", "已保存到数据库:".green(), song_count);
    }
    
    if let Some(output_path) = output {
        crawler.save_to_json(&data, &output_path)?;
        println!("{} {}", "已保存到:".green(), output_path);
    }
    
    Ok(())
}

fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Show => {
            let config = GequConfig::load()?;
            
            #[derive(Tabled)]
            struct ConfigRow {
                #[tabled(rename = "键")]
                key: &'static str,
                #[tabled(rename = "值")]
                value: String,
            }
            
            let rows = vec![
                ConfigRow { key: "cookie", value: if config.cookie.is_empty() { "(未设置)".to_string() } else { format!("{}...", &config.cookie[..20.min(config.cookie.len())]) } },
                ConfigRow { key: "db_path", value: config.db_path },
                ConfigRow { key: "download_dir", value: config.download_dir },
                ConfigRow { key: "output_format", value: config.output_format },
                ConfigRow { key: "timeout", value: config.timeout.to_string() },
            ];
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
            println!("\n配置文件: {:?}", GequConfig::get_config_file());
        }
        
        ConfigCommands::Set { key, value } => {
            let mut config = GequConfig::load()?;
            config.set(&key, &value)?;
            println!("{} {} = {}", "已设置".green(), key, value);
        }
        
        ConfigCommands::Get { key } => {
            let config = GequConfig::load()?;
            if let Some(value) = config.get(&key) {
                println!("{}", value);
            } else {
                println!("{}", "配置项不存在".red());
            }
        }
        
        ConfigCommands::Reset => {
            let mut config = GequConfig::load()?;
            config.reset()?;
            println!("{}", "配置已重置".green());
        }
    }
    
    Ok(())
}