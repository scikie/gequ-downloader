"""
数据模型定义 - 统一的数据结构，供所有爬虫和数据库使用
"""
from dataclasses import dataclass
from datetime import datetime
from typing import Optional


@dataclass
class Singer:
    """歌手信息"""
    name: str
    avatar_url: Optional[str] = None
    songs_url: Optional[str] = None


@dataclass
class Song:
    """歌曲信息"""
    song_id: int
    title: str
    artist: str
    cover_url: Optional[str] = None
    mp3_url: Optional[str] = None
    play_id: Optional[str] = None
    lrc: Optional[str] = None
    extra_url: Optional[str] = None


@dataclass
class RankingItem:
    """排行榜项"""
    ranking_type: str
    rank: int
    item_id: Optional[int] = None
    item_name: Optional[str] = None
    item_type: str = "song"


@dataclass
class SearchKeyword:
    """搜索关键词"""
    keyword: str
    source: str
    rank: Optional[int] = None


@dataclass
class DownloadRecord:
    """下载记录"""
    song_id: int
    file_path: str
    file_size: Optional[int] = None
    downloaded_at: Optional[str] = None


RANKING_TYPES = {
    "singer": "歌手榜",
    "surge": "飙升榜",
    "new": "新歌榜",
    "douyin": "抖音榜",
    "jingdian": "怀旧榜",
    "dianyin": "电音榜",
    "wwdj": "DJ榜",
}

SEARCH_SOURCES = {
    "latest": "最新搜索",
    "hot": "大家都在搜",
}
