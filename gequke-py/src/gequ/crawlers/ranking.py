import json
import re
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

import httpx
from bs4 import BeautifulSoup


@dataclass
class SongRank:
    rank: int
    title: str
    artist: str
    song_id: int
    cover_url: str
    song_url: str


@dataclass
class SingerRank:
    rank: int
    name: str
    avatar_url: str
    songs_url: str


@dataclass
class Pagination:
    current_page: int
    total_pages: int
    total_songs: int
    has_prev: bool
    has_next: bool
    first_page_url: Optional[str]
    prev_page_url: Optional[str]
    next_page_url: Optional[str]
    last_page_url: Optional[str]


@dataclass
class RankingPageData:
    ranking_name: str
    songs: list[SongRank] = None
    singers: list[SingerRank] = None
    pagination: Pagination = None
    
    def __post_init__(self):
        if self.songs is None:
            self.songs = []
        if self.singers is None:
            self.singers = []


class RankingCrawler:
    RANKING_TYPES = {
        "singer": "歌手榜",
        "surge": "飙升榜",
        "new": "新歌榜",
        "douyin": "抖音榜",
        "jingdian": "怀旧榜",
        "dianyin": "电音榜",
        "wwdj": "DJ榜",
    }
    
    def __init__(self, cookie: str = None, user_agent: str = None, timeout: int = 30):
        self.cookie = cookie
        self.user_agent = user_agent or "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        self.timeout = timeout
    
    def _get_headers(self) -> dict:
        return {
            "User-Agent": self.user_agent,
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        }
    
    def _get_cookies(self) -> dict:
        if not self.cookie:
            return {}
        cookies = {}
        for cookie_str in self.cookie.split(";"):
            cookie_str = cookie_str.strip()
            if "=" in cookie_str:
                name, value = cookie_str.split("=", 1)
                cookies[name.strip()] = value.strip()
        return cookies
    
    async def get_ranking_page(self, ranking_type: str, page: int = 1) -> BeautifulSoup:
        if ranking_type not in self.RANKING_TYPES:
            raise ValueError(f"Invalid ranking type: {ranking_type}")
        
        if ranking_type == "singer":
            url = f"https://www.gequke.com/singer/{page}" if page > 1 else "https://www.gequke.com/singer/"
        else:
            url = f"https://www.gequke.com/top/{ranking_type}"
            if page > 1:
                url = f"{url}?page={page}"
        
        async with httpx.AsyncClient(timeout=self.timeout, follow_redirects=True) as client:
            resp = await client.get(
                url,
                headers=self._get_headers(),
                cookies=self._get_cookies()
            )
            resp.raise_for_status()
            return BeautifulSoup(resp.text, "lxml")
    
    def get_ranking_page_from_file(self, filepath: str) -> BeautifulSoup:
        with open(filepath, "r", encoding="utf-8") as f:
            return BeautifulSoup(f.read(), "lxml")
    
    def extract_ranking_name(self, soup: BeautifulSoup) -> tuple[str, int]:
        h1 = soup.find("h1", {"class": "text-light"})
        if h1:
            text = h1.get_text(strip=True)
            match = re.search(r"(.+?)\s*\(共(\d+)条\)", text)
            if match:
                return match.group(1), int(match.group(2))
            return text, 0
        return "未知榜单", 0
    
    def extract_songs(self, soup: BeautifulSoup) -> list[SongRank]:
        songs = []
        table = soup.find("table", {"id": "myTable"})
        
        if not table:
            return songs
        
        tbody = table.find("tbody")
        if not tbody:
            return songs
        
        rows = tbody.find_all("tr")
        
        for row in rows:
            cols = row.find_all("td")
            if len(cols) < 4:
                continue
            
            rank_text = cols[0].get_text(strip=True)
            rank = int(rank_text) if rank_text.isdigit() else 0
            
            img = cols[1].find("img")
            cover_url = img.get("src", "") if img else ""
            
            song_link = cols[2].find("a")
            title = song_link.get_text(strip=True) if song_link else ""
            song_url = song_link.get("href", "") if song_link else ""
            
            song_id = 0
            if song_url:
                match = re.search(r"/song/(\d+)", song_url)
                if match:
                    song_id = int(match.group(1))
            
            artist = cols[3].get_text(strip=True)
            
            songs.append(SongRank(
                rank=rank,
                title=title,
                artist=artist,
                song_id=song_id,
                cover_url=cover_url,
                song_url=song_url
            ))
        
        return songs
    
    def extract_singers(self, soup: BeautifulSoup) -> list[SingerRank]:
        singers = []
        table = soup.find("table", {"id": "myTable"})
        
        if not table:
            return singers
        
        tbody = table.find("tbody")
        if not tbody:
            return singers
        
        rows = tbody.find_all("tr")
        
        for row in rows:
            cols = row.find_all("td")
            if len(cols) < 4:
                continue
            
            rank_text = cols[0].get_text(strip=True)
            rank = int(rank_text) if rank_text.isdigit() else 0
            
            avatar_link = cols[1].find("a")
            img = cols[1].find("img")
            
            avatar_url = img.get("src", "") if img else ""
            songs_url = avatar_link.get("href", "") if avatar_link else ""
            
            name = cols[2].get_text(strip=True)
            
            singers.append(SingerRank(
                rank=rank,
                name=name,
                avatar_url=avatar_url,
                songs_url=songs_url
            ))
        
        return singers
    
    def extract_pagination(self, soup: BeautifulSoup, current_page: int = 1) -> Pagination:
        pagination_nav = soup.find("nav", {"aria-label": "Page navigation"})
        
        if not pagination_nav:
            return Pagination(
                current_page=current_page,
                total_pages=1,
                total_songs=0,
                has_prev=False,
                has_next=False,
                first_page_url=None,
                prev_page_url=None,
                next_page_url=None,
                last_page_url=None
            )
        
        page_items = pagination_nav.find_all("li", {"class": "page-item"})
        
        first_url = None
        prev_url = None
        next_url = None
        last_url = None
        has_prev = False
        has_next = False
        
        for item in page_items:
            link = item.find("a")
            if not link:
                continue
            
            text = link.get_text(strip=True)
            href = link.get("href", "")
            if href:
                href = href.replace('"', '').replace("'", '')
            is_disabled = "disabled" in item.get("class", [])
            
            if "首页" in text:
                first_url = href
            elif "上一页" in text:
                prev_url = href
                has_prev = not is_disabled
            elif "下一页" in text:
                next_url = href
                has_next = not is_disabled
            elif "尾页" in text:
                last_url = href
        
        total_pages = 1
        if last_url:
            match = re.search(r"page=(\d+)", last_url)
            if match:
                total_pages = int(match.group(1))
            else:
                match = re.search(r"/singer/(\d+)", last_url)
                if match:
                    total_pages = int(match.group(1))
        
        ranking_name, total_songs = self.extract_ranking_name(soup)
        
        return Pagination(
            current_page=current_page,
            total_pages=total_pages,
            total_songs=total_songs,
            has_prev=has_prev,
            has_next=has_next,
            first_page_url=first_url,
            prev_page_url=prev_url,
            next_page_url=next_url,
            last_page_url=last_url
        )
    
    def extract_all(self, soup: BeautifulSoup, current_page: int = 1) -> RankingPageData:
        ranking_name, total_songs = self.extract_ranking_name(soup)
        
        if "歌手" in ranking_name:
            singers = self.extract_singers(soup)
            pagination = self.extract_pagination(soup, current_page)
            
            return RankingPageData(
                ranking_name=ranking_name,
                singers=singers,
                pagination=pagination
            )
        else:
            songs = self.extract_songs(soup)
            pagination = self.extract_pagination(soup, current_page)
            
            return RankingPageData(
                ranking_name=ranking_name,
                songs=songs,
                pagination=pagination
            )
    
    def save_to_json(self, data: RankingPageData, filepath: str):
        output_path = Path(filepath)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(asdict(data), f, ensure_ascii=False, indent=2)