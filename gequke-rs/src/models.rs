use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Singer {
    pub name: String,
    pub avatar_url: Option<String>,
    pub songs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub song_id: i64,
    pub title: String,
    pub artist: String,
    pub cover_url: Option<String>,
    pub mp3_url: Option<String>,
    pub play_id: Option<String>,
    pub lrc: Option<String>,
    pub extra_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingItem {
    pub ranking_type: String,
    pub rank: i32,
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub item_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKeyword {
    pub keyword: String,
    pub source: String,
    pub rank: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub song_id: i64,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub downloaded_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub page_type: String,
    pub ranking_type: Option<String>,
    pub search_keyword: Option<String>,
    pub page_number: i32,
    pub url: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageItem {
    pub page_snapshot_id: i64,
    pub item_type: String,
    pub item_id: Option<i64>,
    pub position: i32,
    pub extra_data: Option<String>,
}

pub fn get_ranking_types() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("singer", "歌手榜");
    m.insert("surge", "飙升榜");
    m.insert("new", "新歌榜");
    m.insert("douyin", "抖音榜");
    m.insert("jingdian", "怀旧榜");
    m.insert("dianyin", "电音榜");
    m.insert("wwdj", "DJ榜");
    m
}

pub fn get_page_types() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("homepage", "主页");
    m.insert("ranking", "排行榜");
    m.insert("search", "搜索结果");
    m
}