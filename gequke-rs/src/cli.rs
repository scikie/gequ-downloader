//! 命令行接口模块
//! 
//! 【技术选型：clap】
//! clap是Rust最流行的命令行解析库，特点：
//! - 派生宏（derive macro）定义命令结构，声明式编程
//! - 自动生成帮助信息（--help）
//! - 类型安全：参数类型在编译期检查
//! - 支持子命令、可选参数、默认值、验证规则
//! 
//! 【设计模式：命令模式（Command Pattern）】
//! 每个CLI子命令对应一个处理函数，将请求封装为对象
//! 优点：易于扩展新命令，职责分离清晰

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use tabled::{Table, Tabled, settings::Style};

use crate::config::GequConfig;
use crate::database::Database;
use crate::models::{get_ranking_types, get_page_types, Song, Singer, RankingItem, PageSnapshot, PageItem};
use crate::crawlers::{HomepageCrawler, RankingCrawler, SearchCrawler, DownloadCrawler};

// 【知识点：Parser派生宏】
// #[derive(Parser)] 自动实现命令行参数解析
// 结构体的每个字段对应一个命令行参数
#[derive(Parser)]
#[command(name = "gequ")]           // 程序名称
#[command(about = "歌曲客网站爬虫工具", long_about = None)]  // 简短描述
#[command(version)]                  // 自动添加 --version 选项
pub struct Cli {
    /// 子命令枚举
    /// 
    /// 【知识点：Subcommand派生宏】
    /// 定义互斥的子命令，如：gequ crawl, gequ download
    /// 只能同时指定一个子命令
    #[command(subcommand)]
    command: Commands,
}

/// 顶级命令枚举
/// 
/// 【知识点：Rust枚举】
/// 枚举变体可以携带数据，这是代数数据类型（ADT）的体现
/// 相比C语言的枚举，功能强大得多
#[derive(Subcommand)]
enum Commands {
    /// 爬取相关子命令（嵌套子命令）
    #[command(subcommand)]
    Crawl(CrawlCommands),
    
    /// 下载相关子命令
    #[command(subcommand)]
    Download(DownloadCommands),
    
    /// 数据库相关子命令
    #[command(subcommand)]
    Db(DbCommands),
    
    /// 统计相关子命令
    #[command(subcommand)]
    Stats(StatsCommands),
    
    /// 搜索命令（非子命令形式，直接参数）
    /// 
    /// 【知识点：命令参数定义】
    /// keyword: String - 位置参数（必填）
    /// #[arg(short, long)] - 短选项和长选项
    /// #[arg(long)] - 仅长选项
    /// Option<T> - 可选参数
    Search {
        keyword: String,                    // 搜索关键词（位置参数）
        #[arg(short, long)]
        output: Option<String>,             // -o, --output 输出文件
        #[arg(long)]
        no_db: bool,                        // --no-db 标志（不需要值）
        #[arg(short, long)]
        file: Option<String>,               // -f, --file 从文件读取
    },
    
    /// 配置相关子命令
    #[command(subcommand)]
    Config(ConfigCommands),
    
    /// 显示版本（与#[command(version)]配合）
    Version,
}

/// 爬取子命令
#[derive(Subcommand)]
enum CrawlCommands {
    /// 爬取主页
    Homepage {
        #[arg(short, long)]
        file: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
    
    /// 爬取排行榜
    /// 
    /// 【知识点：参数默认值】
    /// #[arg(default_value = "1")] 设置默认值
    /// 用户未指定时使用该值
    Ranking {
        ranking_type: String,               // 榜单类型
        #[arg(short, long, default_value = "1")]
        page: i32,                          // 页码，默认1
        #[arg(short = 's', long)]           // 自定义短选项为-s
        start_page: Option<i32>,            // 批量爬取起始页
        #[arg(short = 'e', long)]           // 自定义短选项为-e
        end_page: Option<i32>,              // 批量爬取结束页
        #[arg(short, long)]
        file: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
}

/// 下载子命令
#[derive(Subcommand)]
enum DownloadCommands {
    /// 下载单首歌曲
    Song {
        song_id: i64,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_cover: bool,
        #[arg(long)]
        no_db: bool,
    },
    
    /// 批量下载多首歌曲
    /// 
    /// 【知识点：可变数量参数】
    /// Vec<i64> 接收多个同类型参数：gequ download songs 1 2 3
    Songs {
        song_ids: Vec<i64>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        no_db: bool,
    },
}

/// 数据库查询子命令
#[derive(Subcommand)]
enum DbCommands {
    /// 显示统计信息
    Stats,
    
    /// 查询歌手
    Singer {
        name: Option<String>,               // 可选名称，不提供则列出所有
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,                        // 限制数量
    },
    
    /// 查询歌曲
    Song {
        song_id: Option<i64>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
    
    /// 查询下载记录
    Download {
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
    
    /// 查询排行榜
    Ranking {
        ranking_type: Option<String>,
        #[arg(short = 'n', long, default_value = "20")]
        number: i64,
    },
}

/// 统计子命令
#[derive(Subcommand)]
enum StatsCommands {
    /// 歌曲统计
    Song {
        song_id: i64,
        #[arg(long)]
        history: bool,
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
    },
    
    /// 歌手统计
    Singer {
        name: String,
        #[arg(long)]
        history: bool,
        #[arg(short = 'n', long, default_value = "10")]
        number: i64,
    },
    
    /// 热门歌曲统计
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
    
    /// 热门歌手统计
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
    
    /// 历史快照查询
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

/// 配置子命令
#[derive(Subcommand)]
enum ConfigCommands {
    /// 显示当前配置
    Show,
    
    /// 设置配置项
    Set {
        key: String,
        value: String,
    },
    
    /// 获取配置项
    Get {
        key: String,
    },
    
    /// 重置为默认值
    Reset,
}

/// 运行CLI
/// 
/// 【知识点：程序入口函数】
/// pub async fn run() 是CLI模块对外暴露的唯一入口
/// 使用 ? 传播错误，由main函数统一处理
pub async fn run() -> Result<()> {
    // 【知识点：参数解析】
    // Cli::parse() 解析命令行参数
    // 如果参数无效，自动打印帮助信息并退出
    let cli = Cli::parse();
    
    // 【知识点：模式匹配分发】
    // match 枚举变体，调用对应的处理函数
    match cli.command {
        Commands::Crawl(cmd) => handle_crawl(cmd).await?,
        Commands::Download(cmd) => handle_download(cmd).await?,
        Commands::Db(cmd) => handle_db(cmd)?,
        Commands::Stats(cmd) => handle_stats(cmd)?,
        Commands::Search { keyword, output, no_db, file } => 
            handle_search(keyword, output, no_db, file).await?,
        Commands::Config(cmd) => handle_config(cmd)?,
        Commands::Version => println!("gequ version {}", env!("CARGO_PKG_VERSION")),
    }
    
    Ok(())
}

/// 处理爬取命令
/// 
/// 【知识点：异步函数】
/// async fn 定义异步函数，内部可以使用 .await
/// 返回 impl Future<Output = Result<()>>
async fn handle_crawl(cmd: CrawlCommands) -> Result<()> {
    // 加载配置
    let config = GequConfig::load()?;
    
    match cmd {
        // 处理主页爬取
        CrawlCommands::Homepage { file, output, no_db } => {
            // 【知识点：条件表达式】
            // if let Some(...) = ... 是处理Option的惯用方式
            // 比 match 更简洁，适用于单一模式匹配
            let crawler = HomepageCrawler::new(
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            // 【知识点：colored库】
            // .cyan() 方法为字符串添加ANSI颜色
            // 仅终端支持时显示颜色，重定向时自动禁用
            println!("{}", "正在爬取主页...".cyan());
            
            // 从文件或网络获取HTML
            let doc = if let Some(filepath) = file {
                HomepageCrawler::get_homepage_from_file(&filepath)?
            } else {
                crawler.get_homepage().await?
            };
            
            // 提取数据
            let data = crawler.extract_all(&doc);
            
            println!("{}", "成功提取数据".green());
            
            // 【知识点：嵌套结构体定义】
            // 函数内定义临时结构体，用于表格输出
            #[derive(Tabled)]
            struct StatRow {
                #[tabled(rename = "类型")]    // 自定义表头
                r#type: &'static str,        // r#前缀用于关键字作为标识符
                #[tabled(rename = "数量")]
                count: usize,
            }
            
            let rows = vec![
                StatRow { r#type: "最新搜索", count: data.latest_searches.len() },
                StatRow { r#type: "热门关键词", count: data.hot_keywords.len() },
                StatRow { r#type: "热门歌手", count: data.hot_singers.len() },
            ];
            // 【知识点：tabled库】
            // 自动将结构体数据格式化为ASCII表格
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
            
            // 显示详细信息
            if !data.latest_searches.is_empty() {
                println!("\n{}", "最新搜索:".bold());
                // .iter().take(5) 只显示前5条
                for item in data.latest_searches.iter().take(5) {
                    println!("  - {}", item.keyword);
                }
            }
            
            // 保存到数据库（除非指定--no-db）
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
                
                // 批量插入热门歌手
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
            
            // 保存到JSON文件（如果指定--output）
            if let Some(output_path) = output {
                crawler.save_to_json(&data, &output_path)?;
                println!("\n{} {}", "已保存到:".green(), output_path);
            }
        }
        
        // 处理排行榜爬取
        CrawlCommands::Ranking { ranking_type, page, start_page, end_page, file, output, no_db } => {
            let crawler = RankingCrawler::new(
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            // 验证榜单类型
            let ranking_types = crawler.get_ranking_types();
            let ranking_name = ranking_types.get(ranking_type.as_str())
                .ok_or_else(|| anyhow::anyhow!("无效的榜单类型: {}", ranking_type))?;
            
            // 批量爬取多页
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
                
                // 保存所有页面数据到数据库
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
                        
                        // 根据数据类型分别处理歌手榜和歌曲榜
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
                // 单页爬取
                println!("{}", format!("正在爬取 {}...", ranking_name).cyan());
                
                let doc = if let Some(filepath) = file {
                    RankingCrawler::get_ranking_page_from_file(&filepath)?
                } else {
                    crawler.get_ranking_page(&ranking_type, page).await?
                };
                
                let data = crawler.extract_all(&doc, page);
                println!("{}", "成功提取数据".green());
                
                // 显示表格
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
                
                // 根据数据类型显示不同表格
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
                
                // 保存到数据库
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
                
                // 保存到JSON
                if let Some(output_path) = output {
                    crawler.save_to_json(&data, &output_path)?;
                    println!("\n{} {}", "已保存到:".green(), output_path);
                }
            }
        }
    }
    
    Ok(())
}

/// 处理下载命令
async fn handle_download(cmd: DownloadCommands) -> Result<()> {
    let config = GequConfig::load()?;
    
    match cmd {
        // 下载单首歌曲
        DownloadCommands::Song { song_id, output, no_cover, no_db } => {
            // 使用配置中的下载目录或用户指定的目录
            let output_dir = output.unwrap_or_else(|| config.download_dir.clone());
            let crawler = DownloadCrawler::new(
                &output_dir,
                if config.cookie.is_empty() { None } else { Some(config.cookie.clone()) },
                Some(config.user_agent.clone()),
                Some(config.timeout),
            );
            
            println!("{}", format!("正在下载歌曲 {}...", song_id).cyan());
            
            // 执行下载
            let result = crawler.download_song(song_id, !no_cover).await;
            
            if result.success {
                println!("{}", "下载成功".green());
                if let Some(ref path) = result.mp3_path {
                    println!("{}: {}", "音频".green(), path);
                }
                if let Some(ref path) = result.lrc_path {
                    println!("{}: {}", "歌词".green(), path);
                }
                
                // 保存下载记录到数据库
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
                    // 获取文件大小
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
        
        // 批量下载
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
                    
                    // 保存到数据库
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
            
            // 汇总统计
            println!("\n{}: {} 成功, {} 失败", "完成".bold(), success_count, fail_count);
        }
    }
    
    Ok(())
}

/// 处理数据库查询命令
/// 
/// 【知识点：同步函数】
/// 不需要网络IO的操作使用普通fn而非async fn
fn handle_db(cmd: DbCommands) -> Result<()> {
    let config = GequConfig::load()?;
    let db = Database::new(&config.db_path)?;
    
    match cmd {
        // 显示统计信息
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
        
        // 查询歌手详情或列表
        DbCommands::Singer { name, number } => {
            if let Some(singer_name) = name {
                // 查询特定歌手
                if let Some(singer) = db.get_singer_by_name(&singer_name)? {
                    println!("{}", format!("歌手: {}", singer_name).bold());
                    println!("ID: {}", singer["id"]);
                    println!("名称: {}", singer["name"]);
                    println!("头像: {}", singer["avatar_url"].as_str().unwrap_or("-"));
                    println!("歌曲页: {}", singer["songs_url"].as_str().unwrap_or("-"));
                    println!("创建时间: {}", singer["created_at"].as_str().unwrap_or("-"));
                    
                    // 获取出现统计
                    let stats = db.get_singer_appearance_stats(&singer_name)?;
                    println!("\n{}", "出现统计:".bold());
                    println!("  总次数: {}", stats["total_count"]);
                    println!("  主页: {}", stats["homepage_count"]);
                    println!("  排行榜: {}", stats["ranking_count"]);
                } else {
                    println!("{}", format!("未找到歌手: {}", singer_name).red());
                }
            } else {
                // 列出歌手
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
        
        // 查询歌曲详情或列表
        DbCommands::Song { song_id, number } => {
            if let Some(id) = song_id {
                // 查询特定歌曲
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
                // 列出歌曲
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
        
        // 查询下载记录
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
        
        // 查询排行榜
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

/// 处理统计命令
fn handle_stats(cmd: StatsCommands) -> Result<()> {
    let config = GequConfig::load()?;
    let db = Database::new(&config.db_path)?;
    
    match cmd {
        // 歌曲统计详情
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
        
        // 歌手统计详情
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
        
        // 热门歌曲排行
        StatsCommands::TopSongs { number, from, to, last } => {
            // 解析时间参数
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
            
            // enumerate 获取索引作为排名
            let rows: Vec<_> = songs.iter().enumerate().map(|(i, s)| TopSongRow {
                rank: i + 1,
                title: s["title"].as_str().unwrap_or("").to_string(),
                artist: s["artist"].as_str().unwrap_or("").to_string(),
                count: s["appearance_count"].as_i64().unwrap_or(0),
            }).collect();
            
            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        
        // 热门歌手排行
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
        
        // 历史快照查询
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

/// 解析时间参数
/// 
/// 【知识点：函数设计】
/// 纯函数：给定相同输入总是产生相同输出，无副作用
/// 这样的函数易于测试和推理
fn parse_time_args(from: Option<String>, to: Option<String>, last: Option<String>) -> Result<(Option<String>, Option<String>)> {
    // 处理相对时间（如"7d"、"2w"、"1m"）
    if let Some(last_str) = last {
        use regex::Regex;
        // 正则匹配数字+单位
        let re = Regex::new(r"^(\d+)([dDwWmM])$").unwrap();
        
        if let Some(caps) = re.captures(&last_str) {
            let num: i64 = caps[1].parse()?;
            let unit = caps[2].to_lowercase();
            
            use chrono::{Local, Duration};
            let now = Local::now();
            // 根据单位计算时间差
            let delta = match unit.as_str() {
                "d" => Duration::days(num),
                "w" => Duration::weeks(num),
                "m" => Duration::days(num * 30),  // 简化处理，每月30天
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

/// 处理搜索命令
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
    
    // 显示搜索结果表格
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
    
    // 保存到数据库
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
    
    // 保存到JSON
    if let Some(output_path) = output {
        crawler.save_to_json(&data, &output_path)?;
        println!("{} {}", "已保存到:".green(), output_path);
    }
    
    Ok(())
}

/// 处理配置命令
fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        // 显示当前配置
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
        
        // 设置配置项
        ConfigCommands::Set { key, value } => {
            let mut config = GequConfig::load()?;
            config.set(&key, &value)?;
            println!("{} {} = {}", "已设置".green(), key, value);
        }
        
        // 获取配置项
        ConfigCommands::Get { key } => {
            let config = GequConfig::load()?;
            if let Some(value) = config.get(&key) {
                println!("{}", value);
            } else {
                println!("{}", "配置项不存在".red());
            }
        }
        
        // 重置配置
        ConfigCommands::Reset => {
            let mut config = GequConfig::load()?;
            config.reset()?;
            println!("{}", "配置已重置".green());
        }
    }
    
    Ok(())
}

// 【扩展知识：CLI设计原则】
//
// 1. 渐进式披露：
//    - 常用功能简单直观（gequ search 周杰伦）
//    - 高级功能通过选项提供（--start-page, --end-page）
//
// 2. 一致性和惯例：
//    - 使用 -o/--output 表示输出文件
//    - 使用 -n/--number 表示数量限制
//    - 使用 --no-* 前缀表示禁用某功能
//
// 3. 错误处理：
//    - 使用 ? 传播错误
//    - 提供清晰的错误信息
//    - 使用颜色区分成功/失败/警告
//
// 4. 可组合性：
//    - 支持管道（将来可扩展）
//    - 支持JSON输出供脚本使用
