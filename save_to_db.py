"""
数据保存示例 - 展示如何将爬取的数据保存到数据库
"""
import argparse
from pathlib import Path

from homepage_crawler import HomepageCrawler
from ranking_crawler import RankingCrawler
from download_crawler import GequkeDownloader
from database import Database
from models import Singer, Song, RankingItem, SearchKeyword, DownloadRecord, PageSnapshot, PageItem
import json


def save_homepage_data(db: Database, html_file: str = None):
    """保存主页数据到数据库"""
    crawler = HomepageCrawler()
    
    if html_file:
        print(f"从本地文件读取: {html_file}")
        soup = crawler.get_homepage_from_file(html_file)
        url = None
    else:
        print("从网站获取主页...")
        soup = crawler.get_homepage()
        url = "https://www.gequke.com/"
    
    data = crawler.extract_all(soup)
    
    # 创建页面快照
    snapshot = PageSnapshot(
        page_type="homepage",
        url=url,
        title="歌曲客主页"
    )
    snapshot_id = db.insert_page_snapshot(snapshot)
    print(f"已创建页面快照 (ID: {snapshot_id})")
    
    # 保存最新搜索
    keywords = [
        SearchKeyword(keyword=item.keyword, source="latest")
        for item in data.latest_searches
    ]
    count = db.insert_search_keywords(keywords)
    print(f"已保存 {count} 条最新搜索关键词")
    
    # 保存热门搜索到页面条目
    page_items = []
    for item in data.hot_keywords:
        keyword_item = PageItem(
            page_snapshot_id=snapshot_id,
            item_type="keyword",
            position=item.rank,
            extra_data=json.dumps({"keyword": item.keyword, "url": item.url})
        )
        page_items.append(keyword_item)
    count = db.insert_page_items(page_items)
    print(f"已保存 {count} 条热门搜索到页面条目")
    
    # 保存热门歌手（主页只显示前10位）
    page_items = []
    for item in data.hot_singers:
        singer = Singer(name=item.name, songs_url=item.url)
        singer_id = db.insert_singer(singer)
        
        singer_item = PageItem(
            page_snapshot_id=snapshot_id,
            item_type="singer",
            item_id=singer_id,
            position=item.rank,
            extra_data=json.dumps({"songs_url": item.url})
        )
        page_items.append(singer_item)
    
    count = db.insert_page_items(page_items)
    print(f"已保存 {count} 位歌手到页面条目")


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
    snapshot_count = 0
    
    for idx, data in enumerate(all_data):
        page_number = start_page + idx
        
        # 创建页面快照
        snapshot = PageSnapshot(
            page_type="ranking",
            ranking_type=ranking_type,
            page_number=page_number,
            title=data.ranking_name
        )
        snapshot_id = db.insert_page_snapshot(snapshot)
        snapshot_count += 1
        
        page_items = []
        
        if data.singers:
            # 保存歌手榜
            for singer in data.singers:
                s = Singer(name=singer.name, avatar_url=singer.avatar_url, songs_url=singer.songs_url)
                singer_id = db.insert_singer(s)
                singer_count += 1
                
                r = RankingItem(
                    ranking_type=ranking_type,
                    rank=singer.rank,
                    item_name=singer.name,
                    item_type="singer"
                )
                db.insert_ranking_item(r)
                ranking_count += 1
                
                # 添加到页面条目
                singer_item = PageItem(
                    page_snapshot_id=snapshot_id,
                    item_type="singer",
                    item_id=singer_id,
                    position=singer.rank,
                    extra_data=json.dumps({"avatar_url": singer.avatar_url, "songs_url": singer.songs_url})
                )
                page_items.append(singer_item)
        else:
            # 保存歌曲榜
            for song in data.songs:
                s = Song(
                    song_id=song.song_id,
                    title=song.title,
                    artist=song.artist,
                    cover_url=song.cover_url
                )
                song_id = db.insert_song(s)
                song_count += 1
                
                r = RankingItem(
                    ranking_type=ranking_type,
                    rank=song.rank,
                    item_id=song.song_id,
                    item_type="song"
                )
                db.insert_ranking_item(r)
                ranking_count += 1
                
                # 添加到页面条目
                song_item = PageItem(
                    page_snapshot_id=snapshot_id,
                    item_type="song",
                    item_id=song_id,
                    position=song.rank,
                    extra_data=json.dumps({"title": song.title, "artist": song.artist, "cover_url": song.cover_url})
                )
                page_items.append(song_item)
        
        # 批量插入页面条目
        db.insert_page_items(page_items)
    
    print(f"已创建 {snapshot_count} 个页面快照")
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
    parser.add_argument("--song-stats", type=int, help="查询歌曲出现统计 (song_id)")
    parser.add_argument("--singer-stats", type=str, help="查询歌手出现统计 (歌手名)")
    parser.add_argument("--top-songs", action="store_true", help="显示出现次数最多的歌曲")
    parser.add_argument("--top-singers", action="store_true", help="显示出现次数最多的歌手")
    parser.add_argument("--page-history", type=str, choices=["homepage", "ranking"], help="显示页面快照历史")
    
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
        print(f"  页面快照数: {stats['total_page_snapshots']}")
        print(f"  页面条目数: {stats['total_page_items']}")
        return
    
    if args.song_stats:
        stats = db.get_song_appearance_stats(args.song_stats)
        print(f"\n歌曲 {args.song_stats} 统计:")
        print(f"  总出现次数: {stats['total_count']}")
        print(f"  主页出现: {stats['homepage_count']} 次")
        print(f"  排行榜出现: {stats['ranking_count']} 次")
        if stats['appearances']:
            print(f"\n  最近出现记录:")
            for app in stats['appearances'][:5]:
                print(f"    - {app['page_type']} | {app['ranking_type'] or '-'} | 第{app['page_number']}页 | 排名{app['position']} | {app['crawled_at']}")
        return
    
    if args.singer_stats:
        stats = db.get_singer_appearance_stats(args.singer_stats)
        print(f"\n歌手 {args.singer_stats} 统计:")
        print(f"  总出现次数: {stats['total_count']}")
        print(f"  主页出现: {stats['homepage_count']} 次")
        print(f"  排行榜出现: {stats['ranking_count']} 次")
        if stats['appearances']:
            print(f"\n  最近出现记录:")
            for app in stats['appearances'][:5]:
                print(f"    - {app['page_type']} | {app['ranking_type'] or '-'} | 第{app['page_number']}页 | 排名{app['position']} | {app['crawled_at']}")
        return
    
    if args.top_songs:
        songs = db.get_top_appearing_songs(limit=10)
        print("\n出现次数最多的歌曲:")
        for idx, song in enumerate(songs, 1):
            print(f"  {idx}. {song['title']} - {song['artist']} (出现 {song['appearance_count']} 次)")
        return
    
    if args.top_singers:
        singers = db.get_top_appearing_singers(limit=10)
        print("\n出现次数最多的歌手:")
        for idx, singer in enumerate(singers, 1):
            print(f"  {idx}. {singer['name']} (出现 {singer['appearance_count']} 次)")
        return
    
    if args.page_history:
        snapshots = db.get_page_snapshots(page_type=args.page_history, limit=20)
        print(f"\n{args.page_history} 页面快照历史:")
        for snap in snapshots:
            ranking_type = snap['ranking_type'] or '-'
            title = snap['title'] or '-'
            print(f"  ID:{snap['id']} | {snap['crawled_at']} | {ranking_type} | 第{snap['page_number']}页 | {title}")
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
