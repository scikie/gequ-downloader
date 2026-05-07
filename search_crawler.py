"""
搜索爬虫 - 根据关键词搜索歌曲
"""
import json
import re
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

import requests
from bs4 import BeautifulSoup


@dataclass
class SongSearchResult:
    """搜索结果中的歌曲"""
    position: int
    song_id: int
    title: str
    artist: str
    song_url: str


@dataclass
class SearchResult:
    """搜索结果"""
    keyword: str
    total_count: int
    songs: list[SongSearchResult]


class SearchCrawler:
    def __init__(self, cookies: str = None):
        self.session = requests.Session()
        self.session.headers.update({
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
            "Accept-Encoding": "gzip, deflate, br, zdst",
        })
        
        if cookies:
            self._parse_cookies(cookies)
            print(f"使用浏览器Cookie: {self.session.cookies.get_dict()}")
        else:
            print("未提供Cookie，API可能返回403")
    
    def _parse_cookies(self, cookies_str: str):
        for cookie in cookies_str.split(";"):
            cookie = cookie.strip()
            if "=" in cookie:
                name, value = cookie.split("=", 1)
                self.session.cookies.set(name.strip(), value.strip(), domain="www.gequke.com")
    
    def search(self, keyword: str) -> BeautifulSoup:
        url = f"https://www.gequke.com/ss/{keyword}"
        resp = self.session.get(url)
        resp.raise_for_status()
        resp.encoding = "utf-8"
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
        
        print(f"已保存到: {output_path}")


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="歌曲客搜索爬虫")
    parser.add_argument("keyword", type=str, nargs="?", default="清明上", help="搜索关键词")
    parser.add_argument("-c", "--cookies", type=str, help="浏览器Cookie字符串")
    parser.add_argument("-o", "--output", type=str, default="downloads", help="输出目录")
    parser.add_argument("-f", "--file", type=str, help="从本地HTML文件读取（用于测试）")
    
    args = parser.parse_args()
    
    crawler = SearchCrawler(cookies=args.cookies)
    
    if args.file:
        print(f"从本地文件读取: {args.file}")
        soup = crawler.search_from_file(args.file)
    else:
        print(f"正在搜索: {args.keyword}")
        soup = crawler.search(args.keyword)
    
    data = crawler.extract_all(soup)
    
    print(f"\n关键词: {data.keyword}")
    print(f"找到 {data.total_count} 条结果")
    print(f"实际提取 {len(data.songs)} 首")
    
    print(f"\n搜索结果:")
    for song in data.songs:
        print(f"  {song.position}. {song.title} - {song.artist} (ID: {song.song_id})")
    
    output_file = f"{args.output}/{data.keyword}-搜索结果.json"
    crawler.save_to_json(data, output_file)


if __name__ == "__main__":
    main()