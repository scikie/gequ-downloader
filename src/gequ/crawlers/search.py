import json
import re
from dataclasses import dataclass, asdict
from pathlib import Path

import httpx
from bs4 import BeautifulSoup


@dataclass
class SongSearchResult:
    position: int
    song_id: int
    title: str
    artist: str
    song_url: str


@dataclass
class SearchResult:
    keyword: str
    total_count: int
    songs: list[SongSearchResult]


class SearchCrawler:
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
    
    async def search(self, keyword: str) -> BeautifulSoup:
        url = f"https://www.gequke.com/ss/{keyword}"
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.get(
                url,
                headers=self._get_headers(),
                cookies=self._get_cookies()
            )
            resp.raise_for_status()
            return BeautifulSoup(resp.text, "lxml")
    
    def search_from_file(self, filepath: str) -> BeautifulSoup:
        with open(filepath, "r", encoding="utf-8") as f:
            return BeautifulSoup(f.read(), "lxml")
    
    def extract_keyword(self, soup: BeautifulSoup) -> str:
        input_field = soup.find("input", {"id": "s-input-line"})
        if input_field:
            return input_field.get("value", "")
        
        h1 = soup.find("h1", {"class": "navbar-h1"})
        if h1:
            return h1.get_text(strip=True)
        
        return ""
    
    def extract_total_count(self, soup: BeautifulSoup) -> int:
        div = soup.find("div", {"class": "quote-warning"})
        if div:
            text = div.get_text()
            match = re.search(r"共找到\s*(\d+)\s*条", text)
            if match:
                return int(match.group(1))
        
        return 0
    
    def extract_songs(self, soup: BeautifulSoup) -> list[SongSearchResult]:
        songs = []
        table = soup.find("table", {"id": "myTables"})
        
        if not table:
            return songs
        
        tbody = table.find("tbody")
        if not tbody:
            return songs
        
        rows = tbody.find_all("tr")
        
        for row in rows:
            cols = row.find_all("td")
            if len(cols) < 3:
                continue
            
            position_text = cols[0].get_text(strip=True)
            position = int(position_text) if position_text.isdigit() else 0
            
            song_link = cols[1].find("a")
            title = song_link.get_text(strip=True) if song_link else ""
            song_url = song_link.get("href", "") if song_link else ""
            
            song_id = 0
            if song_url:
                match = re.search(r"/song/(\d+)", song_url)
                if match:
                    song_id = int(match.group(1))
            
            artist = cols[2].get_text(strip=True)
            
            songs.append(SongSearchResult(
                position=position,
                song_id=song_id,
                title=title,
                artist=artist,
                song_url=song_url
            ))
        
        return songs
    
    def extract_all(self, soup: BeautifulSoup) -> SearchResult:
        keyword = self.extract_keyword(soup)
        total_count = self.extract_total_count(soup)
        songs = self.extract_songs(soup)
        
        return SearchResult(
            keyword=keyword,
            total_count=total_count,
            songs=songs
        )
    
    def save_to_json(self, data: SearchResult, filepath: str):
        output_path = Path(filepath)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(asdict(data), f, ensure_ascii=False, indent=2)