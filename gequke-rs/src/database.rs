//! 数据库模块
//! 
//! 【技术选型：rusqlite】
//! rusqlite是Rust的SQLite绑定，特点：
//! - bundled特性：自动编译SQLite，无需系统依赖
//! - 安全的内存管理，防止常见的C语言SQLite错误
//! - 支持所有SQLite功能（事务、预编译语句、FTS等）
//! 
//! 【设计模式：Repository模式】
//! Database结构体封装所有数据访问逻辑：
//! - 提供领域友好的API（insert_song, get_singer_by_name）
//! - 隐藏SQL细节，便于更换数据库
//! - 集中处理数据库连接生命周期

use anyhow::{Result, Context};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::models::*;

/// 数据库连接管理器
/// 
/// 【知识点：结构体作为资源管理器】
/// Database本身不持有连接（Connection不是线程安全），
/// 而是管理连接参数，每次操作创建新连接。
/// 这种设计：
/// - 支持多线程并发（每个线程独立连接）
/// - 简化错误恢复（每次操作都是独立的）
/// - 缺点是连接开销较大（SQLite连接轻量，可接受）
pub struct Database {
    db_path: PathBuf,
}

impl Database {
    /// 创建数据库实例并初始化表结构
    /// 
    /// 【知识点：构造函数模式】
    /// new() 是Rust中构造函数的惯例名称（非关键字）
    /// 与Default::default() 不同，new() 可以接受参数
    pub fn new(db_path: &str) -> Result<Self> {
        let path = PathBuf::from(db_path);
        // 确保数据库目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("创建数据库目录失败")?;
        }
        
        let db = Self { db_path: path };
        // 立即初始化表结构（幂等操作，可重复执行）
        db.init_tables()?;
        Ok(db)
    }

    /// 获取数据库连接
    /// 
    /// 【知识点：私有辅助方法】
    /// 使用 fn 而非 pub fn，表示模块私有
    /// 这是封装原则：外部不应直接操作连接
    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .context("连接数据库失败")
    }

    /// 初始化数据库表结构
    /// 
    /// 【SQL最佳实践】
    /// 1. IF NOT EXISTS：幂等性，多次运行不报错
    /// 2. 外键约束：保证数据完整性
    /// 3. 索引：加速查询，但会增加写入开销
    /// 4. TIMESTAMP DEFAULT CURRENT_TIMESTAMP：自动记录时间
    fn init_tables(&self) -> Result<()> {
        let conn = self.get_connection()?;
        
        // execute_batch 执行多条SQL语句
        conn.execute_batch(r#"
            -- 歌手表
            CREATE TABLE IF NOT EXISTS singers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,  -- UNIQUE约束防止重复
                avatar_url TEXT,
                songs_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- 歌曲表
            CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id INTEGER NOT NULL UNIQUE,  -- 业务ID
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                cover_url TEXT,
                mp3_url TEXT,
                play_id TEXT,
                lrc TEXT,
                extra_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- 排行榜记录表
            CREATE TABLE IF NOT EXISTS rankings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ranking_type TEXT NOT NULL,
                rank INTEGER NOT NULL,
                song_id INTEGER,
                singer_id INTEGER,
                page INTEGER DEFAULT 1,
                crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (song_id) REFERENCES songs(id),
                FOREIGN KEY (singer_id) REFERENCES singers(id)
            );

            -- 搜索关键词表
            CREATE TABLE IF NOT EXISTS search_keywords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                keyword TEXT NOT NULL,
                source TEXT NOT NULL,
                rank INTEGER,
                crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- 下载记录表
            CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                downloaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (song_id) REFERENCES songs(id)
            );

            -- 页面快照表（记录历史状态）
            CREATE TABLE IF NOT EXISTS page_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_type TEXT NOT NULL,
                ranking_type TEXT,
                search_keyword TEXT,
                page_number INTEGER DEFAULT 1,
                url TEXT,
                title TEXT,
                crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- 页面条目表（与快照一对多关系）
            CREATE TABLE IF NOT EXISTS page_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_snapshot_id INTEGER NOT NULL,
                item_type TEXT NOT NULL,
                item_id INTEGER,
                position INTEGER DEFAULT 0,
                extra_data TEXT,
                FOREIGN KEY (page_snapshot_id) REFERENCES page_snapshots(id)
            );

            -- 索引优化查询性能
            CREATE INDEX IF NOT EXISTS idx_songs_song_id ON songs(song_id);
            CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist);
            CREATE INDEX IF NOT EXISTS idx_rankings_type ON rankings(ranking_type);
            CREATE INDEX IF NOT EXISTS idx_rankings_rank ON rankings(ranking_type, rank);
            CREATE INDEX IF NOT EXISTS idx_singers_name ON singers(name);
            CREATE INDEX IF NOT EXISTS idx_keywords_keyword ON search_keywords(keyword);
            CREATE INDEX IF NOT EXISTS idx_page_snapshots_type ON page_snapshots(page_type);
            CREATE INDEX IF NOT EXISTS idx_page_snapshots_date ON page_snapshots(crawled_at);
            CREATE INDEX IF NOT EXISTS idx_page_items_snapshot ON page_items(page_snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_page_items_item ON page_items(item_type, item_id);
        "#).context("初始化数据库表失败")?;

        Ok(())
    }

    /// 插入或更新歌手
    /// 
    /// 【知识点：UPSERT操作】
    /// ON CONFLICT(name) DO UPDATE SET ...
    /// SQLite的UPSERT语法：插入时如果冲突则更新
    /// 注意：SQLite 3.24.0+ 支持（2018年发布）
    /// 
    /// 【参数绑定】
    /// params![] 宏自动处理SQL注入防护
    /// 永远不要使用字符串拼接SQL！
    pub fn insert_singer(&self, singer: &Singer) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO singers (name, avatar_url, songs_url)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(name) DO UPDATE SET
                avatar_url = excluded.avatar_url,
                songs_url = excluded.songs_url,
                updated_at = CURRENT_TIMESTAMP
        "#, params![singer.name, singer.avatar_url, singer.songs_url])
            .context("插入歌手失败")?;

        // 查询刚插入/更新的记录ID
        let id: i64 = conn.query_row(
            "SELECT id FROM singers WHERE name = ?1",
            params![singer.name],
            |row| row.get(0)
        ).context("查询歌手ID失败")?;

        Ok(id)
    }

    /// 插入或更新歌曲
    /// 
    /// 【知识点：COALESCE函数】
    /// COALESCE(excluded.cover_url, cover_url)
    /// 优先使用新值，如果新值为NULL则保留旧值
    /// 这样更新时不会用NULL覆盖已有数据
    pub fn insert_song(&self, song: &Song) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO songs (song_id, title, artist, cover_url, mp3_url, play_id, lrc, extra_url)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(song_id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                cover_url = COALESCE(excluded.cover_url, cover_url),
                mp3_url = COALESCE(excluded.mp3_url, mp3_url),
                play_id = COALESCE(excluded.play_id, play_id),
                lrc = COALESCE(excluded.lrc, lrc),
                extra_url = COALESCE(excluded.extra_url, extra_url),
                updated_at = CURRENT_TIMESTAMP
        "#, params![
            song.song_id, song.title, song.artist, song.cover_url,
            song.mp3_url, song.play_id, song.lrc, song.extra_url
        ]).context("插入歌曲失败")?;

        let id: i64 = conn.query_row(
            "SELECT id FROM songs WHERE song_id = ?1",
            params![song.song_id],
            |row| row.get(0)
        ).context("查询歌曲ID失败")?;

        Ok(id)
    }

    /// 插入排行榜条目
    /// 
    /// 【业务逻辑】
    /// 先查询关联的歌曲/歌手表获取内部ID，
    /// 再插入rankings表建立关联
    pub fn insert_ranking_item(&self, item: &RankingItem) -> Result<i64> {
        let conn = self.get_connection()?;
        
        let mut song_db_id: Option<i64> = None;
        let mut singer_db_id: Option<i64> = None;

        // 根据item_type查询对应表的ID
        if item.item_type == "song" {
            if let Some(song_id) = item.item_id {
                song_db_id = conn.query_row(
                    "SELECT id FROM songs WHERE song_id = ?1",
                    params![song_id],
                    |row| row.get(0)
                ).ok();  // .ok() 将Result转为Option，忽略错误
            }
        } else if item.item_type == "singer" {
            if let Some(ref name) = item.item_name {
                singer_db_id = conn.query_row(
                    "SELECT id FROM singers WHERE name = ?1",
                    params![name],
                    |row| row.get(0)
                ).ok();
            }
        }

        conn.execute(r#"
            INSERT INTO rankings (ranking_type, rank, song_id, singer_id)
            VALUES (?1, ?2, ?3, ?4)
        "#, params![item.ranking_type, item.rank, song_db_id, singer_db_id])
            .context("插入排行榜失败")?;

        Ok(conn.last_insert_rowid())
    }

    /// 插入搜索关键词
    pub fn insert_search_keyword(&self, keyword: &SearchKeyword) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO search_keywords (keyword, source, rank)
            VALUES (?1, ?2, ?3)
        "#, params![keyword.keyword, keyword.source, keyword.rank])
            .context("插入搜索关键词失败")?;

        Ok(conn.last_insert_rowid())
    }

    /// 插入下载记录
    pub fn insert_download_record(&self, record: &DownloadRecord) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO downloads (song_id, file_path, file_size)
            VALUES (?1, ?2, ?3)
        "#, params![record.song_id, record.file_path, record.file_size])
            .context("插入下载记录失败")?;

        Ok(conn.last_insert_rowid())
    }

    /// 插入页面快照
    pub fn insert_page_snapshot(&self, snapshot: &PageSnapshot) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO page_snapshots (page_type, ranking_type, search_keyword, page_number, url, title)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#, params![
            snapshot.page_type, snapshot.ranking_type, snapshot.search_keyword,
            snapshot.page_number, snapshot.url, snapshot.title
        ]).context("插入页面快照失败")?;

        Ok(conn.last_insert_rowid())
    }

    /// 插入单条页面条目
    pub fn insert_page_item(&self, item: &PageItem) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO page_items (page_snapshot_id, item_type, item_id, position, extra_data)
            VALUES (?1, ?2, ?3, ?4, ?5)
        "#, params![
            item.page_snapshot_id, item.item_type, item.item_id, item.position, item.extra_data
        ]).context("插入页面条目失败")?;

        Ok(conn.last_insert_rowid())
    }

    /// 批量插入页面条目
    /// 
    /// 【知识点：事务】
    /// 虽然这里没有显式使用事务，但SQLite每个INSERT都是原子操作
    /// 批量插入应该考虑包装在事务中以提高性能
    pub fn insert_page_items(&self, items: &[PageItem]) -> Result<usize> {
        let mut count = 0;
        for item in items {
            self.insert_page_item(item)?;
            count += 1;
        }
        Ok(count)
    }

    /// 获取数据库统计信息
    /// 
    /// 【知识点：serde_json::Value】
    /// 动态JSON类型，用于返回不确定结构的数据
    /// 类似JavaScript的对象或Python的字典
    pub fn get_stats(&self) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        
        // query_row 查询单行结果，空参数用 []
        let total_singers: i64 = conn.query_row("SELECT COUNT(*) FROM singers", [], |row| row.get(0))?;
        let total_songs: i64 = conn.query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?;
        let total_rankings: i64 = conn.query_row("SELECT COUNT(*) FROM rankings", [], |row| row.get(0))?;
        let total_keywords: i64 = conn.query_row("SELECT COUNT(*) FROM search_keywords", [], |row| row.get(0))?;
        let total_downloads: i64 = conn.query_row("SELECT COUNT(*) FROM downloads", [], |row| row.get(0))?;
        let total_page_snapshots: i64 = conn.query_row("SELECT COUNT(*) FROM page_snapshots", [], |row| row.get(0))?;
        let total_page_items: i64 = conn.query_row("SELECT COUNT(*) FROM page_items", [], |row| row.get(0))?;

        // json! 宏创建JSON对象
        Ok(serde_json::json!({
            "total_singers": total_singers,
            "total_songs": total_songs,
            "total_rankings": total_rankings,
            "total_keywords": total_keywords,
            "total_downloads": total_downloads,
            "total_page_snapshots": total_page_snapshots,
            "total_page_items": total_page_items,
        }))
    }

    /// 根据song_id查询歌曲
    /// 
    /// 【知识点：query_row与错误处理】
    /// query_row返回Result，.ok()转为Option
    /// 这样能优雅处理"记录不存在"的情况
    pub fn get_song_by_id(&self, song_id: i64) -> Result<Option<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        // prepare 编译SQL语句，可重复使用
        let mut stmt = conn.prepare(r#"
            SELECT id, song_id, title, artist, cover_url, created_at
            FROM songs WHERE song_id = ?1
        "#)?;
        
        // query_row 执行查询并映射结果
        let result = stmt.query_row(params![song_id], |row| {
            // json! 宏创建JSON对象
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "song_id": row.get::<_, i64>(1)?,
                "title": row.get::<_, String>(2)?,
                "artist": row.get::<_, String>(3)?,
                "cover_url": row.get::<_, Option<String>>(4)?,
                "created_at": row.get::<_, Option<String>>(5)?,
            }))
        }).ok();

        Ok(result)
    }

    /// 根据名称查询歌手
    pub fn get_singer_by_name(&self, name: &str) -> Result<Option<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT id, name, avatar_url, songs_url, created_at
            FROM singers WHERE name = ?1
        "#)?;
        
        let result = stmt.query_row(params![name], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "avatar_url": row.get::<_, Option<String>>(2)?,
                "songs_url": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, Option<String>>(4)?,
            }))
        }).ok();

        Ok(result)
    }

    /// 获取所有歌手
    /// 
    /// 【知识点：query_map遍历结果】
    /// query_map返回迭代器，惰性处理每行数据
    /// 适合处理大量结果集，避免一次性加载到内存
    pub fn get_all_singers(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT id, name, avatar_url, songs_url, created_at
            FROM singers ORDER BY created_at DESC LIMIT ?1
        "#)?;
        
        // query_map 返回行的迭代器
        let rows = stmt.query_map(params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "avatar_url": row.get::<_, Option<String>>(2)?,
                "songs_url": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, Option<String>>(4)?,
            }))
        })?;

        // 收集所有结果
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 获取所有歌曲
    pub fn get_all_songs(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT id, song_id, title, artist, cover_url, created_at
            FROM songs ORDER BY created_at DESC LIMIT ?1
        "#)?;
        
        let rows = stmt.query_map(params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "song_id": row.get::<_, i64>(1)?,
                "title": row.get::<_, String>(2)?,
                "artist": row.get::<_, String>(3)?,
                "cover_url": row.get::<_, Option<String>>(4)?,
                "created_at": row.get::<_, Option<String>>(5)?,
            }))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 获取所有下载记录（带歌曲信息）
    /// 
    /// 【知识点：SQL JOIN】
    /// LEFT JOIN 保留downloads表所有记录，即使songs表无匹配
    /// 使用表别名（d, s）简化SQL
    pub fn get_all_downloads(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT d.id, d.song_id, d.file_path, d.file_size, d.downloaded_at,
                   s.song_id as real_song_id, s.title, s.artist
            FROM downloads d
            LEFT JOIN songs s ON d.song_id = s.song_id
            ORDER BY d.downloaded_at DESC LIMIT ?1
        "#)?;
        
        let rows = stmt.query_map(params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "song_id": row.get::<_, i64>(1)?,
                "file_path": row.get::<_, String>(2)?,
                "file_size": row.get::<_, Option<i64>>(3)?,
                "downloaded_at": row.get::<_, Option<String>>(4)?,
                "real_song_id": row.get::<_, Option<i64>>(5)?,
                "title": row.get::<_, Option<String>>(6)?,
                "artist": row.get::<_, Option<String>>(7)?,
            }))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 获取排行榜记录
    /// 
    /// 【知识点：模式匹配与动态SQL】
    /// 使用match处理可选的ranking_type参数
    /// 两种情况下构造不同的SQL和参数
    pub fn get_all_rankings(&self, ranking_type: Option<&str>, limit: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let results = match ranking_type {
            Some(r_type) => {
                let mut stmt = conn.prepare(r#"
                    SELECT r.id, r.ranking_type, r.rank, r.crawled_at,
                           s.song_id, s.title, s.artist, sg.name as singer_name
                    FROM rankings r
                    LEFT JOIN songs s ON r.song_id = s.id
                    LEFT JOIN singers sg ON r.singer_id = sg.id
                    WHERE r.ranking_type = ?1
                    ORDER BY r.crawled_at DESC, r.rank ASC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![r_type, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "ranking_type": row.get::<_, String>(1)?,
                        "rank": row.get::<_, i32>(2)?,
                        "crawled_at": row.get::<_, Option<String>>(3)?,
                        "song_id": row.get::<_, Option<i64>>(4)?,
                        "title": row.get::<_, Option<String>>(5)?,
                        "artist": row.get::<_, Option<String>>(6)?,
                        "singer_name": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            None => {
                let mut stmt = conn.prepare(r#"
                    SELECT r.id, r.ranking_type, r.rank, r.crawled_at,
                           s.song_id, s.title, s.artist, sg.name as singer_name
                    FROM rankings r
                    LEFT JOIN songs s ON r.song_id = s.id
                    LEFT JOIN singers sg ON r.singer_id = sg.id
                    ORDER BY r.crawled_at DESC, r.rank ASC LIMIT ?1
                "#)?;
                let rows = stmt.query_map(params![limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "ranking_type": row.get::<_, String>(1)?,
                        "rank": row.get::<_, i32>(2)?,
                        "crawled_at": row.get::<_, Option<String>>(3)?,
                        "song_id": row.get::<_, Option<i64>>(4)?,
                        "title": row.get::<_, Option<String>>(5)?,
                        "artist": row.get::<_, Option<String>>(6)?,
                        "singer_name": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
        };

        results.context("查询排行榜失败")
    }

    /// 获取歌曲出现统计
    /// 
    /// 【知识点：子查询】
    /// 使用嵌套SELECT从songs表获取内部ID
    /// 再用该ID统计page_items中的出现次数
    pub fn get_song_appearance_stats(&self, song_id: i64) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        
        let total_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items 
            WHERE item_type = 'song' AND item_id = (
                SELECT id FROM songs WHERE song_id = ?1
            )
        "#, params![song_id], |row| row.get(0))?;

        // JOIN查询统计不同类型页面的出现次数
        let homepage_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items pi
            JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
            WHERE pi.item_type = 'song' AND pi.item_id = (
                SELECT id FROM songs WHERE song_id = ?1
            ) AND ps.page_type = 'homepage'
        "#, params![song_id], |row| row.get(0))?;

        let ranking_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items pi
            JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
            WHERE pi.item_type = 'song' AND pi.item_id = (
                SELECT id FROM songs WHERE song_id = ?1
            ) AND ps.page_type = 'ranking'
        "#, params![song_id], |row| row.get(0))?;

        Ok(serde_json::json!({
            "song_id": song_id,
            "total_count": total_count,
            "homepage_count": homepage_count,
            "ranking_count": ranking_count,
        }))
    }

    /// 获取歌手出现统计
    pub fn get_singer_appearance_stats(&self, singer_name: &str) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        
        let total_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items 
            WHERE item_type = 'singer' AND item_id = (
                SELECT id FROM singers WHERE name = ?1
            )
        "#, params![singer_name], |row| row.get(0))?;

        let homepage_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items pi
            JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
            WHERE pi.item_type = 'singer' AND pi.item_id = (
                SELECT id FROM singers WHERE name = ?1
            ) AND ps.page_type = 'homepage'
        "#, params![singer_name], |row| row.get(0))?;

        let ranking_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items pi
            JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
            WHERE pi.item_type = 'singer' AND pi.item_id = (
                SELECT id FROM singers WHERE name = ?1
            ) AND ps.page_type = 'ranking'
        "#, params![singer_name], |row| row.get(0))?;

        Ok(serde_json::json!({
            "singer_name": singer_name,
            "total_count": total_count,
            "homepage_count": homepage_count,
            "ranking_count": ranking_count,
        }))
    }

    /// 获取热门歌曲（出现次数最多）
    /// 
    /// 【知识点：复杂模式匹配】
    /// 处理date_from和date_to四种组合情况
    /// 每种情况构造不同的SQL条件
    pub fn get_top_appearing_songs(&self, limit: i64, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        match (date_from, date_to) {
            (Some(from), Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT s.song_id, s.title, s.artist, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at >= ?1 AND ps.crawled_at <= ?2
                    GROUP BY s.song_id ORDER BY appearance_count DESC LIMIT ?3
                "#)?;
                let rows = stmt.query_map(params![from, to, limit], |row| {
                    Ok(serde_json::json!({
                        "song_id": row.get::<_, i64>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "artist": row.get::<_, String>(2)?,
                        "appearance_count": row.get::<_, i64>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (Some(from), None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT s.song_id, s.title, s.artist, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at >= ?1
                    GROUP BY s.song_id ORDER BY appearance_count DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![from, limit], |row| {
                    Ok(serde_json::json!({
                        "song_id": row.get::<_, i64>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "artist": row.get::<_, String>(2)?,
                        "appearance_count": row.get::<_, i64>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT s.song_id, s.title, s.artist, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at <= ?1
                    GROUP BY s.song_id ORDER BY appearance_count DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![to, limit], |row| {
                    Ok(serde_json::json!({
                        "song_id": row.get::<_, i64>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "artist": row.get::<_, String>(2)?,
                        "appearance_count": row.get::<_, i64>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT s.song_id, s.title, s.artist, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    GROUP BY s.song_id ORDER BY appearance_count DESC LIMIT ?1
                "#)?;
                let rows = stmt.query_map(params![limit], |row| {
                    Ok(serde_json::json!({
                        "song_id": row.get::<_, i64>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "artist": row.get::<_, String>(2)?,
                        "appearance_count": row.get::<_, i64>(3)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
        }.context("查询热门歌曲失败")
    }

    /// 获取热门歌手
    pub fn get_top_appearing_singers(&self, limit: i64, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        match (date_from, date_to) {
            (Some(from), Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT sg.name, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at >= ?1 AND ps.crawled_at <= ?2
                    GROUP BY sg.name ORDER BY appearance_count DESC LIMIT ?3
                "#)?;
                let rows = stmt.query_map(params![from, to, limit], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "appearance_count": row.get::<_, i64>(1)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (Some(from), None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT sg.name, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at >= ?1
                    GROUP BY sg.name ORDER BY appearance_count DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![from, limit], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "appearance_count": row.get::<_, i64>(1)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT sg.name, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    WHERE ps.crawled_at <= ?1
                    GROUP BY sg.name ORDER BY appearance_count DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![to, limit], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "appearance_count": row.get::<_, i64>(1)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT sg.name, COUNT(*) as appearance_count
                    FROM page_items pi
                    JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                    JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                    GROUP BY sg.name ORDER BY appearance_count DESC LIMIT ?1
                "#)?;
                let rows = stmt.query_map(params![limit], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "appearance_count": row.get::<_, i64>(1)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
        }.context("查询热门歌手失败")
    }

    /// 获取页面快照列表
    /// 
    /// 【知识点：参数组合爆炸】
    /// 三个可选参数(page_type, date_from, date_to)产生8种组合
    /// 实际项目中可考虑使用Builder模式简化
    pub fn get_page_snapshots(&self, page_type: Option<&str>, limit: i64, date_from: Option<&str>, date_to: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        match (page_type, date_from, date_to) {
            (Some(pt), Some(from), Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE page_type = ?1 AND crawled_at >= ?2 AND crawled_at <= ?3
                    ORDER BY crawled_at DESC LIMIT ?4
                "#)?;
                let rows = stmt.query_map(params![pt, from, to, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (Some(pt), Some(from), None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE page_type = ?1 AND crawled_at >= ?2
                    ORDER BY crawled_at DESC LIMIT ?3
                "#)?;
                let rows = stmt.query_map(params![pt, from, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (Some(pt), None, Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE page_type = ?1 AND crawled_at <= ?2
                    ORDER BY crawled_at DESC LIMIT ?3
                "#)?;
                let rows = stmt.query_map(params![pt, to, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (Some(pt), None, None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE page_type = ?1
                    ORDER BY crawled_at DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![pt, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, Some(from), Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE crawled_at >= ?1 AND crawled_at <= ?2
                    ORDER BY crawled_at DESC LIMIT ?3
                "#)?;
                let rows = stmt.query_map(params![from, to, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, Some(from), None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE crawled_at >= ?1
                    ORDER BY crawled_at DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![from, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, None, Some(to)) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    WHERE crawled_at <= ?1
                    ORDER BY crawled_at DESC LIMIT ?2
                "#)?;
                let rows = stmt.query_map(params![to, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
            (None, None, None) => {
                let mut stmt = conn.prepare(r#"
                    SELECT * FROM page_snapshots 
                    ORDER BY crawled_at DESC LIMIT ?1
                "#)?;
                let rows = stmt.query_map(params![limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "page_type": row.get::<_, String>(1)?,
                        "ranking_type": row.get::<_, Option<String>>(2)?,
                        "search_keyword": row.get::<_, Option<String>>(3)?,
                        "page_number": row.get::<_, i32>(4)?,
                        "url": row.get::<_, Option<String>>(5)?,
                        "title": row.get::<_, Option<String>>(6)?,
                        "crawled_at": row.get::<_, Option<String>>(7)?,
                    }))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            }
        }.context("查询页面快照失败")
    }
}

// 【扩展知识：数据库事务】
// 
// 示例：批量插入使用事务提高性能
// pub fn insert_page_items_batch(&self, items: &[PageItem]) -> Result<usize> {
//     let mut conn = self.get_connection()?;
//     let tx = conn.transaction()?;  // 开始事务
//     
//     let mut count = 0;
//     for item in items {
//         tx.execute("INSERT ...", params![...])?;
//         count += 1;
//     }
//     
//     tx.commit()?;  // 提交事务
//     Ok(count)
// }
// 
// 事务的优势：
// - 原子性：全部成功或全部回滚
// - 性能：批量提交减少IO次数
// - 一致性：事务内数据对外不可见
