"""
数据结构：
- SearchKeyword - 最新搜索（关键词、链接）
- RankedKeyword - 热门搜索排行（排名、关键词、链接）
- HotSinger - 热门歌手排行（排名、歌手名、链接）
- HomepageData - 主页数据汇总
使用方式：
# 从网站爬取
uv run homepage_crawler.py
# 从本地HTML测试
uv run homepage_crawler.py -f "downloads/xxx.html"
# 指定输出路径
uv run homepage_crawler.py -o path/to/output.json
已成功提取并保存到 downloads/homepage.json
"""
import json
from dataclasses import dataclass, asdict
from pathlib import Path

import requests
from bs4 import BeautifulSoup


@dataclass
class SearchKeyword:
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
    latest_searches: list[SearchKeyword]
    hot_keywords: list[RankedKeyword]
    hot_singers: list[HotSinger]


class HomepageCrawler:
    def __init__(self, cookies: str = None):
        self.session = requests.Session()
        self.session.headers.update({
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
            "Accept-Encoding": "gzip, deflate, br, zstd",
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
    
    def get_homepage(self) -> BeautifulSoup:
        url = "https://www.gequke.com/"
        resp = self.session.get(url)
        resp.raise_for_status()
        resp.encoding = "utf-8"
        return BeautifulSoup(resp.text, "lxml")
    
    def get_homepage_from_file(self, filepath: str) -> BeautifulSoup:
        with open(filepath, "r", encoding="utf-8") as f:
            return BeautifulSoup(f.read(), "lxml")
    
    def extract_latest_searches(self, soup: BeautifulSoup) -> list[SearchKeyword]:
        latest_searches = []
        singerlist_div = soup.find("div", {"class": "ilingku_singerlist"})
        
        if singerlist_div:
            links = singerlist_div.find_all("a")
            for link in links:
                keyword = link.get_text(strip=True)
                url = link.get("href", "")
                latest_searches.append(SearchKeyword(keyword=keyword, url=url))
        
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
        
        print(f"已保存到: {output_path}")


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="歌曲客主页爬虫")
    parser.add_argument("-c", "--cookies", type=str, help="浏览器Cookie字符串")
    parser.add_argument("-o", "--output", type=str, default="downloads/homepage.json", help="输出JSON文件路径")
    parser.add_argument("-f", "--file", type=str, help="从本地HTML文件读取（用于测试）")
    
    args = parser.parse_args()
    
    crawler = HomepageCrawler(cookies=args.cookies)
    
    if args.file:
        print(f"从本地文件读取: {args.file}")
        soup = crawler.get_homepage_from_file(args.file)
    else:
        print("从网站获取主页...")
        soup = crawler.get_homepage()
    
    print("正在提取数据...")
    data = crawler.extract_all(soup)
    
    print(f"\n最新搜索 ({len(data.latest_searches)} 个):")
    for item in data.latest_searches[:5]:
        print(f"  - {item.keyword}")
    
    print(f"\n大家都在搜 ({len(data.hot_keywords)} 个):")
    for item in data.hot_keywords[:5]:
        print(f"  {item.rank}. {item.keyword}")
    
    print(f"\n热门歌手榜 ({len(data.hot_singers)} 个):")
    for item in data.hot_singers[:5]:
        print(f"  {item.rank}. {item.name}")
    
    crawler.save_to_json(data, args.output)


if __name__ == "__main__":
    main()
