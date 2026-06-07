"""
## 支持的榜单类型：
- singer - 歌手榜 
- surge - 飙升榜
- new - 新歌榜
- douyin - 抖音榜
- jingdian - 怀旧榜
- dianyin - 电音榜
- wwdj - DJ榜
## 使用示例：
```bash
cd playground
# 爬取歌手榜第1页
uv run ranking_crawler.py singer -p 1
# 爬取歌手榜第1-10页
uv run ranking_crawler.py singer -s 1 -e 10
```
"""

import json
import re
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Optional

import requests
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
    
    def get_ranking_page(self, ranking_type: str, page: int = 1) -> BeautifulSoup:
        if ranking_type not in self.RANKING_TYPES:
            raise ValueError(f"Invalid ranking type: {ranking_type}. Valid types: {list(self.RANKING_TYPES.keys())}")
        
        if ranking_type == "singer":
            if page == 1:
                url = "https://www.gequke.com/singer"
            else:
                url = f"https://www.gequke.com/singer/{page}"
        else:
            url = f"https://www.gequke.com/top/{ranking_type}"
            if page > 1:
                url = f"{url}?page={page}"
        
        resp = self.session.get(url)
        resp.raise_for_status()
        resp.encoding = "utf-8"
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
        
        print(f"已保存到: {output_path}")
    
    def crawl_multiple_pages(self, ranking_type: str, start_page: int = 1, end_page: int = None, output_dir: str = "downloads") -> list[RankingPageData]:
        all_data = []
        
        print(f"开始爬取 {self.RANKING_TYPES.get(ranking_type, ranking_type)}...")
        
        first_soup = self.get_ranking_page(ranking_type, start_page)
        first_data = self.extract_all(first_soup, start_page)
        all_data.append(first_data)
        
        if end_page is None:
            end_page = first_data.pagination.total_pages
        
        end_page = min(end_page, first_data.pagination.total_pages)
        
        for page in range(start_page + 1, end_page + 1):
            print(f"正在爬取第 {page} 页...")
            soup = self.get_ranking_page(ranking_type, page)
            data = self.extract_all(soup, page)
            all_data.append(data)
        
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)
        
        ranking_name = self.RANKING_TYPES.get(ranking_type, ranking_type)
        output_file = output_path / f"{ranking_name}_page_{start_page}-{end_page}.json"
        
        if ranking_type == "singer":
            all_singers = []
            for data in all_data:
                all_singers.extend(data.singers)
            
            combined_data = {
                "ranking_name": ranking_name,
                "start_page": start_page,
                "end_page": end_page,
                "total_singers": len(all_singers),
                "singers": [asdict(singer) for singer in all_singers]
            }
            
            with open(output_file, "w", encoding="utf-8") as f:
                json.dump(combined_data, f, ensure_ascii=False, indent=2)
            
            print(f"已保存 {len(all_singers)} 位歌手到: {output_file}")
        else:
            all_songs = []
            for data in all_data:
                all_songs.extend(data.songs)
            
            combined_data = {
                "ranking_name": ranking_name,
                "start_page": start_page,
                "end_page": end_page,
                "total_songs": len(all_songs),
                "songs": [asdict(song) for song in all_songs]
            }
            
            with open(output_file, "w", encoding="utf-8") as f:
                json.dump(combined_data, f, ensure_ascii=False, indent=2)
            
            print(f"已保存 {len(all_songs)} 首歌曲到: {output_file}")
        
        return all_data


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="歌曲客排行榜爬虫")
    parser.add_argument("ranking_type", type=str, nargs="?", default="new",
                        help=f"榜单类型: {', '.join(RankingCrawler.RANKING_TYPES.keys())}")
    parser.add_argument("-p", "--page", type=int, default=1, help="爬取指定页码（默认第1页）")
    parser.add_argument("-s", "--start-page", type=int, help="多页爬取起始页")
    parser.add_argument("-e", "--end-page", type=int, help="多页爬取结束页")
    parser.add_argument("-c", "--cookies", type=str, help="浏览器Cookie字符串")
    parser.add_argument("-o", "--output", type=str, default="downloads", help="输出目录")
    parser.add_argument("-f", "--file", type=str, help="从本地HTML文件读取（用于测试）")
    
    args = parser.parse_args()
    
    crawler = RankingCrawler(cookies=args.cookies)
    
    if args.file:
        print(f"从本地文件读取: {args.file}")
        soup = crawler.get_ranking_page_from_file(args.file)
        data = crawler.extract_all(soup, args.page)
        
        print(f"\n榜单: {data.ranking_name}")
        if data.singers:
            print(f"歌手数: {len(data.singers)}")
        else:
            print(f"歌曲数: {len(data.songs)}")
        print(f"页码: {data.pagination.current_page}/{data.pagination.total_pages}")
        print(f"总数: {data.pagination.total_songs}")
        
        if data.singers:
            print(f"\n前5位歌手:")
            for singer in data.singers[:5]:
                print(f"  {singer.rank}. {singer.name}")
        else:
            print(f"\n前5首歌曲:")
            for song in data.songs[:5]:
                print(f"  {song.rank}. {song.title} - {song.artist} (ID: {song.song_id})")
        
        output_file = f"{args.output}/{data.ranking_name}_page_{args.page}.json"
        crawler.save_to_json(data, output_file)
    
    elif args.start_page and args.end_page:
        all_data = crawler.crawl_multiple_pages(
            args.ranking_type, 
            args.start_page, 
            args.end_page, 
            args.output
        )
        
        if args.ranking_type == "singer":
            total_items = sum(len(data.singers) for data in all_data)
            print(f"\n共爬取 {len(all_data)} 页，{total_items} 位歌手")
        else:
            total_items = sum(len(data.songs) for data in all_data)
            print(f"\n共爬取 {len(all_data)} 页，{total_items} 首歌曲")
    
    else:
        print(f"正在爬取 {crawler.RANKING_TYPES.get(args.ranking_type, args.ranking_type)} 第 {args.page} 页...")
        soup = crawler.get_ranking_page(args.ranking_type, args.page)
        data = crawler.extract_all(soup, args.page)
        
        print(f"\n榜单: {data.ranking_name}")
        if data.singers:
            print(f"当前页歌手数: {len(data.singers)}")
        else:
            print(f"当前页歌曲数: {len(data.songs)}")
        print(f"页码: {data.pagination.current_page}/{data.pagination.total_pages}")
        print(f"总数: {data.pagination.total_songs}")
        
        if data.singers:
            print(f"\n前5位歌手:")
            for singer in data.singers[:5]:
                print(f"  {singer.rank}. {singer.name}")
        else:
            print(f"\n前5首歌曲:")
            for song in data.songs[:5]:
                print(f"  {song.rank}. {song.title} - {song.artist} (ID: {song.song_id})")
        
        if data.pagination.has_next:
            print(f"\n下一页: {data.pagination.next_page_url}")
        if data.pagination.has_prev:
            print(f"上一页: {data.pagination.prev_page_url}")
        
        output_file = f"{args.output}/{data.ranking_name}_page_{args.page}.json"
        crawler.save_to_json(data, output_file)


if __name__ == "__main__":
    main()