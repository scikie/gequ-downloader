//! 数据模型模块
//! 
//! 【知识点：serde序列化库】
//! serde是Rust的事实标准序列化框架，支持JSON、YAML、TOML等多种格式
//! 
//! 常用派生宏：
//! - #[derive(Serialize)] - 将结构体序列化为数据格式
//! - #[derive(Deserialize)] - 从数据格式反序列化为结构体
//! - #[derive(Debug)] - 实现Debug trait，支持 {:?} 格式化输出
//! - #[derive(Clone)] - 实现Clone trait，支持显式深拷贝
//! 
//! serde的设计哲学：
//! - 零成本抽象：序列化/反序列化没有运行时开销
//! - 声明式：通过属性宏控制行为，无需手动实现

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 【知识点：Rust结构体定义】
// pub struct 定义公开的结构体，字段默认私有
// Option<T> 表示可选值，有 Some(T) 或 None 两种状态
// 相比null指针安全，强制调用者处理空值情况

/// 歌手信息
/// 
/// 【设计模式：领域模型】
/// 每个结构体对应业务领域中的一个实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Singer {
    pub name: String,              // 歌手名称
    pub avatar_url: Option<String>, // 头像URL，可能不存在
    pub songs_url: Option<String>,  // 歌曲列表页面URL
}

/// 歌曲信息
/// 
/// 【知识点：Rust整数类型】
/// i64: 有符号64位整数，范围约 ±9×10^18
/// 选择i64而非i32的原因：
/// - 数据库中song_id可能很大
/// - 与SQLite的INTEGER类型对齐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub song_id: i64,              // 歌曲唯一ID
    pub title: String,             // 歌曲标题
    pub artist: String,            // 歌手名
    pub cover_url: Option<String>, // 封面图片URL
    pub mp3_url: Option<String>,   // 音频文件URL
    pub play_id: Option<String>,   // 播放ID（用于API调用）
    pub lrc: Option<String>,       // 歌词内容
    pub extra_url: Option<String>, // 备用链接
}

/// 排行榜条目
/// 
/// 【知识点：i32 vs i64的选择】
/// 排名(rank)使用i32因为：
/// - 排名范围有限（通常<10^6）
/// - 内存占用更小（4字节vs8字节）
/// - 与数据库INTEGER兼容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingItem {
    pub ranking_type: String,      // 榜单类型（如"singer"、"surge"）
    pub rank: i32,                 // 排名位置
    pub item_id: Option<i64>,      // 关联的歌曲或歌手ID
    pub item_name: Option<String>, // 项目名称
    pub item_type: String,         // 条目类型（"song"或"singer"）
}

/// 搜索关键词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchKeyword {
    pub keyword: String,           // 搜索词
    pub source: String,            // 来源页面
    pub rank: Option<i32>,         // 排名（热门搜索）
}

/// 下载记录
/// 
/// 【知识点：时间戳存储】
/// 使用String存储时间而非chrono::DateTime的原因：
/// 1. 简化JSON序列化
/// 2. 与SQLite的TEXT类型兼容
/// 3. 避免时区处理复杂性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub song_id: i64,              // 歌曲ID
    pub file_path: String,         // 本地文件路径
    pub file_size: Option<i64>,    // 文件大小（字节）
    pub downloaded_at: Option<String>, // 下载时间
}

/// 页面快照
/// 
/// 【设计模式：快照模式（Snapshot Pattern）】
/// 记录某个时间点的页面状态，用于：
/// - 数据追溯：查看历史抓取记录
/// - 变化检测：比较不同时期的数据变化
/// - 数据恢复：从快照重建状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub page_type: String,              // 页面类型（homepage/ranking/search）
    pub ranking_type: Option<String>,   // 榜单类型（如果是排行榜页面）
    pub search_keyword: Option<String>, // 搜索词（如果是搜索页面）
    pub page_number: i32,               // 页码
    pub url: Option<String>,            // 页面URL
    pub title: Option<String>,          // 页面标题
}

/// 页面条目（与快照关联）
/// 
/// 【数据库设计：一对多关系】
/// page_snapshot (1) -----> (*) page_item
/// 通过page_snapshot_id外键关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageItem {
    pub page_snapshot_id: i64,     // 所属快照ID
    pub item_type: String,         // 条目类型
    pub item_id: Option<i64>,      // 关联的歌曲/歌手ID
    pub position: i32,             // 在页面中的位置
    pub extra_data: Option<String>, // 额外JSON数据
}

// 【知识点：静态生命周期】
// &'static str 表示字符串字面量，生命周期与程序相同
// 存储在程序二进制文件的只读数据段
// 适合配置字符串、错误消息等常量

/// 获取榜单类型映射
/// 
/// 【知识点：HashMap】
/// HashMap<K, V> 是Rust的标准哈希表实现
/// - 平均O(1)的插入、查找、删除
/// - 基于Robin Hood哈希算法，实际性能优秀
/// - 注意：遍历顺序不固定（与BTreeMap不同）
/// 
/// 【设计决策】
/// 使用&'static str而非String：
/// - 这些键值对是编译期已知的常量
/// - 避免堆内存分配
/// - 零成本运行时
pub fn get_ranking_types() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // insert方法返回Option<V>，插入已存在键时返回旧值
    m.insert("singer", "歌手榜");
    m.insert("surge", "飙升榜");
    m.insert("new", "新歌榜");
    m.insert("douyin", "抖音榜");
    m.insert("jingdian", "怀旧榜");
    m.insert("dianyin", "电音榜");
    m.insert("wwdj", "DJ榜");
    m
}

/// 获取页面类型映射
/// 
/// 【知识点：函数返回值的优化（RVO）】
/// Rust会自动应用返回值优化，避免不必要的拷贝
/// HashMap在这里被直接构造在调用者的栈空间
pub fn get_page_types() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("homepage", "主页");
    m.insert("ranking", "排行榜");
    m.insert("search", "搜索结果");
    m
}

// 【扩展知识：序列化自定义】
// 如果需要自定义序列化行为，可以使用serde属性：
// 
// #[derive(Serialize)]
// struct User {
//     #[serde(rename = "userName")]  // 更改JSON字段名
//     name: String,
//     
//     #[serde(skip_serializing_if = "Option::is_none")]  // None时跳过
//     email: Option<String>,
//     
//     #[serde(default)]  // 反序列化时使用Default::default()
//     age: u8,
// }
