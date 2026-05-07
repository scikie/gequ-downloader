import json
import re
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

import httpx
from bs4 import BeautifulSoup


@dataclass
class SearchKeywordItem:
    keyword: str
    url: str


@dataclass
class RankedKeyword:
    rank: int
    keyword: str
    url: str


@dataclass
class HotSinger:
    rank: int
    name: str
    url: str


@dataclass
class HomepageData:
    latest_searches: list[SearchKeywordItem]
    hot_keywords: list[RankedKeyword]
    hot_singers: list[HotSinger]


class HomepageCrawler:
    def __init__(self, cookie: str = None, user_agent: str = None, timeout: int = 30):
        self.cookie = cookie
        self.user_agent = user_agent or "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        self.timeout = timeout
        
    def _get_headers(self) -> dict:
        headers = {
            "User-Agent": self.user_agent,
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        }
        return headers
    
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
    
    async def get_homepage(self) -> BeautifulSoup:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.get(
                "https://www.gequke.com/",
                headers=self._get_headers(),
                cookies=self._get_cookies()
            )
            resp.raise_for_status()
            return BeautifulSoup(resp.text, "lxml")
    
    def get_homepage_from_file(self, filepath: str) -> BeautifulSoup:
        with open(filepath, "r", encoding="utf-8") as f:
            return BeautifulSoup(f.read(), "lxml")
    
    def extract_latest_searches(self, soup: BeautifulSoup) -> list[SearchKeywordItem]:
        latest_searches = []
        singerlist_div = soup.find("div", {"class": "ilingku_singerlist"})
        
        if singerlist_div:
            links = singerlist_div.find_all("a")
            for link in links:
                keyword = link.get_text(strip=True)
                url = link.get("href", "")
                latest_searches.append(SearchKeywordItem(keyword=keyword, url=url))
        
        return latest_searches
    
    def extract_hot_keywords(self, soup: BeautifulSoup) -> list[RankedKeyword]:
        hot_keywords = []
        tables = soup.find_all("table", {"class": "table"})
        
        for table in tables:
            card_body = table.find_parent("div", {"class": "card-body"})
            if card_body:
                header = card_body.find("h6", {"class": "card-title"})
                if header and "大家都在搜" in header.get_text():
                    tbody = table.find("tbody")
                    if tbody:
                        rows = tbody.find_all("tr")
                        for row in rows:
                            rank_badge = row.find("span", {"class": "badge"})
                            keyword_link = row.find("a")
                            
                            if rank_badge and keyword_link:
                                rank_text = rank_badge.get_text(strip=True)
                                rank = int(rank_text)
                                keyword = keyword_link.get_text(strip=True)
                                url = keyword_link.get("href", "")
                                hot_keywords.append(RankedKeyword(rank=rank, keyword=keyword, url=url))
                    break
        
        return hot_keywords
    
    def extract_hot_singers(self, soup: BeautifulSoup) -> list[HotSinger]:
        hot_singers = []
        tables = soup.find_all("table", {"class": "table"})
        
        for table in tables:
            header = table.find_previous("h6")
            if header and "热门歌手榜" in header.get_text():
                rows = table.find("tbody").find_all("tr")
                for row in rows:
                    rank_badge = row.find("span", {"class": "badge"})
                    singer_link = row.find("a")
                    
                    if rank_badge and singer_link:
                        rank = int(rank_badge.get_text(strip=True))
                        name = singer_link.get_text(strip=True)
                        url = singer_link.get("href", "")
                        hot_singers.append(HotSinger(rank=rank, name=name, url=url))
                break
        
        return hot_singers
    
    def extract_all(self, soup: BeautifulSoup) -> HomepageData:
        latest_searches = self.extract_latest_searches(soup)
        hot_keywords = self.extract_hot_keywords(soup)
        hot_singers = self.extract_hot_singers(soup)
        
        return HomepageData(
            latest_searches=latest_searches,
            hot_keywords=hot_keywords,
            hot_singers=hot_singers
        )
    
    def save_to_json(self, data: HomepageData, filepath: str):
        output_path = Path(filepath)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(asdict(data), f, ensure_ascii=False, indent=2)