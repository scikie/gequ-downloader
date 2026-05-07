"""
数据保存示例 - 展示如何将爬取的数据保存到数据库
"""
import argparse
from pathlib import Path

from homepage_crawler import HomepageCrawler
from ranking_crawler import RankingCrawler
from download_crawler import GequkeDownloader
from database import Database
from models import Singer, Song, RankingItem, SearchKeyword, DownloadRecord


def save_homepage_data(db: Database, html_file: str = None):
    """保存主页数据到数据库"""
    crawler = HomepageCrawler()
    
    if html_file:
        print(f"从本地文件读取: {html_file}")
        soup = crawler.get_homepage_from_file(html_file)
    else:
        print("从网站获取主页...")
        soup = crawler.get_homepage()
    
    data = crawler.extract_all(soup)
    
    # 保存最新搜索
    keywords = [
        SearchKeyword(keyword=item.keyword, source="latest")
        for item in data.latest_searches
    ]
    count = db.insert_search_keywords(keywords)
    print(f"已保存 {count} 条最新搜索关键词")
    
    # 保存热门搜索
    keywords = [
        SearchKeyword(keyword=item.keyword, source="hot", rank=item.rank)
        for item in data.hot_keywords
    ]
    count = db.insert_search_keywords(keywords)
    print(f"已保存 {count} 条热门搜索关键词")
    
    # 保存热门歌手（主页只显示前10位）
    singers = [
        Singer(name=item.name, songs_url=item.url)
        for item in data.hot_singers
    ]
    count = db.insert_singers(singers)
    print(f"已保存 {count} 位歌手信息")


def save_ranking_data(db: Database, ranking_type: str, start_page: int = 1, end_page: int = None, html_file: str = None):
    """保存排行榜数据到数据库"""
    crawler = RankingCrawler()
    
    if html_file:
        print(f"从本地文件读取: {html_file}")
        soup = crawler.get_ranking_page_from_file(html_file)
        all_data = [crawler.extract_all(soup, start_page)]
    else:
        ranking_name = crawler.RANKING_TYPES.get(ranking_type, ranking_type)
        print(f"正在爬取 {ranking_name}...")
        
        first_soup = crawler.get_ranking_page(ranking_type, start_page)
        first_data = crawler.extract_all(first_soup, start_page)
        
        if end_page is None:
            end_page = min(5, first_data.pagination.total_pages)
        
        all_data = [first_data]
        
        for page in range(start_page + 1, end_page + 1):
            print(f"正在爬取第 {page} 页...")
            soup = crawler.get_ranking_page(ranking_type, page)
            data = crawler.extract_all(soup, page)
            all_data.append(data)
    
    # 保存数据
    song_count = 0
    singer_count = 0
    ranking_count = 0
    
    for data in all_data:
        if data.singers:
            # 保存歌手榜
            for singer in data.singers:
                s = Singer(name=singer.name, avatar_url=singer.avatar_url, songs_url=singer.songs_url)
                db.insert_singer(s)
                singer_count += 1
                
                r = RankingItem(
                    ranking_type=ranking_type,
                    rank=singer.rank,
                    item_name=singer.name,
                    item_type="singer"
                )
                db.insert_ranking_item(r)
                ranking_count += 1
        else:
            # 保存歌曲榜
            for song in data.songs:
                s = Song(
                    song_id=song.song_id,
                    title=song.title,
                    artist=song.artist,
                    cover_url=song.cover_url
                )
                db.insert_song(s)
                song_count += 1
                
                r = RankingItem(
                    ranking_type=ranking_type,
                    rank=song.rank,
                    item_id=song.song_id,
                    item_type="song"
                )
                db.insert_ranking_item(r)
                ranking_count += 1
    
    print(f"已保存 {singer_count} 位歌手")
    print(f"已保存 {song_count} 首歌曲")
    print(f"已保存 {ranking_count} 条排行记录")


def save_download_record(db: Database, song_id: int, file_path: str, file_size: int = None):
    """保存下载记录到数据库"""
    record = DownloadRecord(
        song_id=song_id,
        file_path=file_path,
        file_size=file_size
    )
    record_id = db.insert_download_record(record)
    
    if record_id > 0:
        print(f"已保存下载记录 (ID: {record_id})")
    else:
        print("保存失败：歌曲不存在")


def main():
    parser = argparse.ArgumentParser(description="数据保存示例")
    parser.add_argument("-d", "--db", type=str, default="gequke.db", help="数据库文件路径")
    parser.add_argument("--homepage", action="store_true", help="保存主页数据")
    parser.add_argument("--homepage-file", type=str, help="从本地HTML保存主页数据")
    parser.add_argument("--ranking", type=str, help="保存指定榜单数据")
    parser.add_argument("--start-page", type=int, default=1, help="起始页码")
    parser.add_argument("--end-page", type=int, help="结束页码")
    parser.add_argument("--ranking-file", type=str, help="从本地HTML保存榜单数据")
    parser.add_argument("--stats", action="store_true", help="显示数据库统计")
    
    args = parser.parse_args()
    
    db = Database(args.db)
    
    if args.stats:
        stats = db.get_stats()
        print("\n数据库统计:")
        print(f"  歌手数: {stats['total_singers']}")
        print(f"  歌曲数: {stats['total_songs']}")
        print(f"  排行记录数: {stats['total_rankings']}")
        print(f"  搜索关键词数: {stats['total_keywords']}")
        print(f"  下载记录数: {stats['total_downloads']}")
        return
    
    if args.homepage or args.homepage_file:
        save_homepage_data(db, args.homepage_file)
    
    if args.ranking or args.ranking_file:
        ranking_type = args.ranking or "new"
        save_ranking_data(
            db, 
            ranking_type, 
            args.start_page, 
            args.end_page,
            args.ranking_file
        )


if __name__ == "__main__":
    main()
