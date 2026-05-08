use anyhow::{Result, Context};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::models::*;

pub struct Database {
    db_path: PathBuf,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = PathBuf::from(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("创建数据库目录失败")?;
        }
        
        let db = Self { db_path: path };
        db.init_tables()?;
        Ok(db)
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .context("连接数据库失败")
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.get_connection()?;
        
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS singers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                avatar_url TEXT,
                songs_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id INTEGER NOT NULL UNIQUE,
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

            CREATE TABLE IF NOT EXISTS search_keywords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                keyword TEXT NOT NULL,
                source TEXT NOT NULL,
                rank INTEGER,
                crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id INTEGER NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                downloaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (song_id) REFERENCES songs(id)
            );

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

            CREATE TABLE IF NOT EXISTS page_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_snapshot_id INTEGER NOT NULL,
                item_type TEXT NOT NULL,
                item_id INTEGER,
                position INTEGER DEFAULT 0,
                extra_data TEXT,
                FOREIGN KEY (page_snapshot_id) REFERENCES page_snapshots(id)
            );

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

        let id: i64 = conn.query_row(
            "SELECT id FROM singers WHERE name = ?1",
            params![singer.name],
            |row| row.get(0)
        ).context("查询歌手ID失败")?;

        Ok(id)
    }

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

    pub fn insert_ranking_item(&self, item: &RankingItem) -> Result<i64> {
        let conn = self.get_connection()?;
        
        let mut song_db_id: Option<i64> = None;
        let mut singer_db_id: Option<i64> = None;

        if item.item_type == "song" {
            if let Some(song_id) = item.item_id {
                song_db_id = conn.query_row(
                    "SELECT id FROM songs WHERE song_id = ?1",
                    params![song_id],
                    |row| row.get(0)
                ).ok();
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

    pub fn insert_search_keyword(&self, keyword: &SearchKeyword) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO search_keywords (keyword, source, rank)
            VALUES (?1, ?2, ?3)
        "#, params![keyword.keyword, keyword.source, keyword.rank])
            .context("插入搜索关键词失败")?;

        Ok(conn.last_insert_rowid())
    }

    pub fn insert_download_record(&self, record: &DownloadRecord) -> Result<i64> {
        let conn = self.get_connection()?;
        
        conn.execute(r#"
            INSERT INTO downloads (song_id, file_path, file_size)
            VALUES (?1, ?2, ?3)
        "#, params![record.song_id, record.file_path, record.file_size])
            .context("插入下载记录失败")?;

        Ok(conn.last_insert_rowid())
    }

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

    pub fn insert_page_items(&self, items: &[PageItem]) -> Result<usize> {
        let mut count = 0;
        for item in items {
            self.insert_page_item(item)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn get_stats(&self) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        
        let total_singers: i64 = conn.query_row("SELECT COUNT(*) FROM singers", [], |row| row.get(0))?;
        let total_songs: i64 = conn.query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?;
        let total_rankings: i64 = conn.query_row("SELECT COUNT(*) FROM rankings", [], |row| row.get(0))?;
        let total_keywords: i64 = conn.query_row("SELECT COUNT(*) FROM search_keywords", [], |row| row.get(0))?;
        let total_downloads: i64 = conn.query_row("SELECT COUNT(*) FROM downloads", [], |row| row.get(0))?;
        let total_page_snapshots: i64 = conn.query_row("SELECT COUNT(*) FROM page_snapshots", [], |row| row.get(0))?;
        let total_page_items: i64 = conn.query_row("SELECT COUNT(*) FROM page_items", [], |row| row.get(0))?;

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

    pub fn get_song_by_id(&self, song_id: i64) -> Result<Option<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT id, song_id, title, artist, cover_url, created_at
            FROM songs WHERE song_id = ?1
        "#)?;
        
        let result = stmt.query_row(params![song_id], |row| {
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

    pub fn get_all_singers(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(r#"
            SELECT id, name, avatar_url, songs_url, created_at
            FROM singers ORDER BY created_at DESC LIMIT ?1
        "#)?;
        
        let rows = stmt.query_map(params![limit], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "avatar_url": row.get::<_, Option<String>>(2)?,
                "songs_url": row.get::<_, Option<String>>(3)?,
                "created_at": row.get::<_, Option<String>>(4)?,
            }))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

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

    pub fn get_song_appearance_stats(&self, song_id: i64) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        
        let total_count: i64 = conn.query_row(r#"
            SELECT COUNT(*) FROM page_items 
            WHERE item_type = 'song' AND item_id = (
                SELECT id FROM songs WHERE song_id = ?1
            )
        "#, params![song_id], |row| row.get(0))?;

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