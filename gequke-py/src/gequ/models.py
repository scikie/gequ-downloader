from dataclasses import dataclass
from typing import Optional


@dataclass
class Singer:
    name: str
    avatar_url: Optional[str] = None
    songs_url: Optional[str] = None


@dataclass
class Song:
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
    ranking_type: str
    rank: int
    item_id: Optional[int] = None
    item_name: Optional[str] = None
    item_type: str = "song"


@dataclass
class SearchKeyword:
    keyword: str
    source: str
    rank: Optional[int] = None


@dataclass
class DownloadRecord:
    song_id: int
    file_path: str
    file_size: Optional[int] = None
    downloaded_at: Optional[str] = None


@dataclass
class PageSnapshot:
    page_type: str
    ranking_type: Optional[str] = None
    search_keyword: Optional[str] = None
    page_number: int = 1
    url: Optional[str] = None
    title: Optional[str] = None


@dataclass
class PageItem:
    page_snapshot_id: int
    item_type: str
    item_id: Optional[int] = None
    position: int = 0
    extra_data: Optional[str] = None


@dataclass
class SearchRecord:
    keyword: str
    total_count: int = 0


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

PAGE_TYPES = {
    "homepage": "主页",
    "ranking": "排行榜",
    "search": "搜索结果",
}
