"""
SQLite 数据库管理模块
"""
import sqlite3
from pathlib import Path
from typing import Optional, List
from contextlib import contextmanager

from models import Singer, Song, RankingItem, SearchKeyword, DownloadRecord, PageSnapshot, PageItem, SearchRecord, RANKING_TYPES, SEARCH_SOURCES


class Database:
    def __init__(self, db_path: str = "gequke.db"):
        self.db_path = Path(db_path)
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_tables()
    
    @contextmanager
    def get_connection(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        try:
            yield conn
            conn.commit()
        except Exception as e:
            conn.rollback()
            raise e
        finally:
            conn.close()
    
    def _init_tables(self):
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS singers (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    avatar_url TEXT,
                    songs_url TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
            """)
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS songs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    song_id INTEGER NOT NULL UNIQUE,
                    title TEXT NOT NULL,
                    artist TEXT NOT NULL,
                    cover_url TEXT,
                    mp3_url TEXT,
                    play_id TEXT,
                    lrc TEXT,
                    extra_url TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
            """)
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS rankings (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ranking_type TEXT NOT NULL,
                    rank INTEGER NOT NULL,
                    song_id INTEGER,
                    singer_id INTEGER,
                    page INTEGER DEFAULT 1,
                    crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(ranking_type, rank, crawled_at),
                    FOREIGN KEY (song_id) REFERENCES songs(id),
                    FOREIGN KEY (singer_id) REFERENCES singers(id)
                )
            """)
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS search_keywords (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    keyword TEXT NOT NULL,
                    source TEXT NOT NULL,
                    rank INTEGER,
                    crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(keyword, source, crawled_at)
                )
            """)
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS downloads (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    song_id INTEGER NOT NULL,
                    file_path TEXT NOT NULL,
                    file_size INTEGER,
                    downloaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (song_id) REFERENCES songs(id)
                )
            """)
            
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_songs_song_id ON songs(song_id)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_rankings_type ON rankings(ranking_type)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_rankings_rank ON rankings(ranking_type, rank)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_singers_name ON singers(name)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_keywords_keyword ON search_keywords(keyword)")
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS page_snapshots (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    page_type TEXT NOT NULL,
                    ranking_type TEXT,
                    search_keyword TEXT,
                    page_number INTEGER DEFAULT 1,
                    url TEXT,
                    title TEXT,
                    crawled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    UNIQUE(page_type, ranking_type, search_keyword, page_number, crawled_at)
                )
            """)
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS page_items (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    page_snapshot_id INTEGER NOT NULL,
                    item_type TEXT NOT NULL,
                    item_id INTEGER,
                    position INTEGER DEFAULT 0,
                    extra_data TEXT,
                    FOREIGN KEY (page_snapshot_id) REFERENCES page_snapshots(id)
                )
            """)
            
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_page_snapshots_type ON page_snapshots(page_type)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_page_snapshots_date ON page_snapshots(crawled_at)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_page_items_snapshot ON page_items(page_snapshot_id)")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_page_items_item ON page_items(item_type, item_id)")
    
    def insert_singer(self, singer: Singer) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO singers (name, avatar_url, songs_url)
                VALUES (?, ?, ?)
                ON CONFLICT(name) DO UPDATE SET
                    avatar_url = excluded.avatar_url,
                    songs_url = excluded.songs_url,
                    updated_at = CURRENT_TIMESTAMP
            """, (singer.name, singer.avatar_url, singer.songs_url))
            
            cursor.execute("SELECT id FROM singers WHERE name = ?", (singer.name,))
            return cursor.fetchone()[0]
    
    def insert_singers(self, singers: List[Singer]) -> int:
        count = 0
        for singer in singers:
            self.insert_singer(singer)
            count += 1
        return count
    
    def insert_song(self, song: Song) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO songs (song_id, title, artist, cover_url, mp3_url, play_id, lrc, extra_url)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(song_id) DO UPDATE SET
                    title = excluded.title,
                    artist = excluded.artist,
                    cover_url = COALESCE(excluded.cover_url, cover_url),
                    mp3_url = COALESCE(excluded.mp3_url, mp3_url),
                    play_id = COALESCE(excluded.play_id, play_id),
                    lrc = COALESCE(excluded.lrc, lrc),
                    extra_url = COALESCE(excluded.extra_url, extra_url),
                    updated_at = CURRENT_TIMESTAMP
            """, (song.song_id, song.title, song.artist, song.cover_url, 
                  song.mp3_url, song.play_id, song.lrc, song.extra_url))
            
            cursor.execute("SELECT id FROM songs WHERE song_id = ?", (song.song_id,))
            return cursor.fetchone()[0]
    
    def insert_songs(self, songs: List[Song]) -> int:
        count = 0
        for song in songs:
            self.insert_song(song)
            count += 1
        return count
    
    def insert_ranking_item(self, item: RankingItem) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            song_db_id = None
            singer_db_id = None
            
            if item.item_type == "song" and item.item_id:
                cursor.execute("SELECT id FROM songs WHERE song_id = ?", (item.item_id,))
                result = cursor.fetchone()
                if result:
                    song_db_id = result[0]
            elif item.item_type == "singer" and item.item_name:
                cursor.execute("SELECT id FROM singers WHERE name = ?", (item.item_name,))
                result = cursor.fetchone()
                if result:
                    singer_db_id = result[0]
            
            cursor.execute("""
                INSERT INTO rankings (ranking_type, rank, song_id, singer_id)
                VALUES (?, ?, ?, ?)
            """, (item.ranking_type, item.rank, song_db_id, singer_db_id))
            
            return cursor.lastrowid
    
    def insert_ranking_items(self, items: List[RankingItem]) -> int:
        count = 0
        for item in items:
            self.insert_ranking_item(item)
            count += 1
        return count
    
    def insert_search_keyword(self, keyword: SearchKeyword) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO search_keywords (keyword, source, rank)
                VALUES (?, ?, ?)
            """, (keyword.keyword, keyword.source, keyword.rank))
            return cursor.lastrowid
    
    def insert_search_keywords(self, keywords: List[SearchKeyword]) -> int:
        count = 0
        for keyword in keywords:
            self.insert_search_keyword(keyword)
            count += 1
        return count
    
    def insert_download_record(self, record: DownloadRecord) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            cursor.execute("SELECT id FROM songs WHERE song_id = ?", (record.song_id,))
            result = cursor.fetchone()
            song_db_id = result[0] if result else None
            
            if not song_db_id:
                return -1
            
            cursor.execute("""
                INSERT INTO downloads (song_id, file_path, file_size, downloaded_at)
                VALUES (?, ?, ?, ?)
            """, (song_db_id, record.file_path, record.file_size, 
                  record.downloaded_at or "CURRENT_TIMESTAMP"))
            
            return cursor.lastrowid
    
    def get_song_by_id(self, song_id: int) -> Optional[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM songs WHERE song_id = ?", (song_id,))
            row = cursor.fetchone()
            return dict(row) if row else None
    
    def get_singer_by_name(self, name: str) -> Optional[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM singers WHERE name = ?", (name,))
            row = cursor.fetchone()
            return dict(row) if row else None
    
    def get_ranking_by_type(self, ranking_type: str, limit: int = 100) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT r.*, s.title, s.artist, s.song_id, sg.name as singer_name
                FROM rankings r
                LEFT JOIN songs s ON r.song_id = s.id
                LEFT JOIN singers sg ON r.singer_id = sg.id
                WHERE r.ranking_type = ?
                ORDER BY r.crawled_at DESC, r.rank ASC
                LIMIT ?
            """, (ranking_type, limit))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_search_keywords(self, source: str = None, limit: int = 100) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            if source:
                cursor.execute("""
                    SELECT * FROM search_keywords 
                    WHERE source = ?
                    ORDER BY crawled_at DESC, rank ASC
                    LIMIT ?
                """, (source, limit))
            else:
                cursor.execute("""
                    SELECT * FROM search_keywords 
                    ORDER BY crawled_at DESC, rank ASC
                    LIMIT ?
                """, (limit,))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_download_history(self, limit: int = 100) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT d.*, s.title, s.artist, s.song_id
                FROM downloads d
                JOIN songs s ON d.song_id = s.id
                ORDER BY d.downloaded_at DESC
                LIMIT ?
            """, (limit,))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_stats(self) -> dict:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            stats = {}
            
            cursor.execute("SELECT COUNT(*) FROM singers")
            stats['total_singers'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM songs")
            stats['total_songs'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM rankings")
            stats['total_rankings'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM search_keywords")
            stats['total_keywords'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM downloads")
            stats['total_downloads'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM page_snapshots")
            stats['total_page_snapshots'] = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM page_items")
            stats['total_page_items'] = cursor.fetchone()[0]
            
            return stats
    
    def insert_page_snapshot(self, snapshot: PageSnapshot) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO page_snapshots (page_type, ranking_type, search_keyword, page_number, url, title)
                VALUES (?, ?, ?, ?, ?, ?)
            """, (snapshot.page_type, snapshot.ranking_type, snapshot.search_keyword, 
                  snapshot.page_number, snapshot.url, snapshot.title))
            return cursor.lastrowid
    
    def insert_page_item(self, item: PageItem) -> int:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO page_items (page_snapshot_id, item_type, item_id, position, extra_data)
                VALUES (?, ?, ?, ?, ?)
            """, (item.page_snapshot_id, item.item_type, item.item_id, 
                  item.position, item.extra_data))
            return cursor.lastrowid
    
    def insert_page_items(self, items: List[PageItem]) -> int:
        count = 0
        for item in items:
            self.insert_page_item(item)
            count += 1
        return count
    
    def get_page_snapshots(self, page_type: str = None, limit: int = 100) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            if page_type:
                cursor.execute("""
                    SELECT * FROM page_snapshots 
                    WHERE page_type = ?
                    ORDER BY crawled_at DESC
                    LIMIT ?
                """, (page_type, limit))
            else:
                cursor.execute("""
                    SELECT * FROM page_snapshots 
                    ORDER BY crawled_at DESC
                    LIMIT ?
                """, (limit,))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_page_items_by_snapshot(self, snapshot_id: int) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT pi.*, s.title, s.artist, s.song_id, sg.name as singer_name
                FROM page_items pi
                LEFT JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                LEFT JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                WHERE pi.page_snapshot_id = ?
                ORDER BY pi.position ASC
            """, (snapshot_id,))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_song_appearance_stats(self, song_id: int) -> dict:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            stats = {
                'song_id': song_id,
                'homepage_count': 0,
                'ranking_count': 0,
                'total_count': 0,
                'appearances': []
            }
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items 
                WHERE item_type = 'song' AND item_id = (
                    SELECT id FROM songs WHERE song_id = ?
                )
            """, (song_id,))
            stats['total_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'song' AND pi.item_id = (
                    SELECT id FROM songs WHERE song_id = ?
                ) AND ps.page_type = 'homepage'
            """, (song_id,))
            stats['homepage_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'song' AND pi.item_id = (
                    SELECT id FROM songs WHERE song_id = ?
                ) AND ps.page_type = 'ranking'
            """, (song_id,))
            stats['ranking_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT ps.*, pi.position
                FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'song' AND pi.item_id = (
                    SELECT id FROM songs WHERE song_id = ?
                )
                ORDER BY ps.crawled_at DESC
                LIMIT 20
            """, (song_id,))
            stats['appearances'] = [dict(row) for row in cursor.fetchall()]
            
            return stats
    
    def get_singer_appearance_stats(self, singer_name: str) -> dict:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            
            stats = {
                'singer_name': singer_name,
                'homepage_count': 0,
                'ranking_count': 0,
                'total_count': 0,
                'appearances': []
            }
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items 
                WHERE item_type = 'singer' AND item_id = (
                    SELECT id FROM singers WHERE name = ?
                )
            """, (singer_name,))
            stats['total_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'singer' AND pi.item_id = (
                    SELECT id FROM singers WHERE name = ?
                ) AND ps.page_type = 'homepage'
            """, (singer_name,))
            stats['homepage_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT COUNT(*) FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'singer' AND pi.item_id = (
                    SELECT id FROM singers WHERE name = ?
                ) AND ps.page_type = 'ranking'
            """, (singer_name,))
            stats['ranking_count'] = cursor.fetchone()[0]
            
            cursor.execute("""
                SELECT ps.*, pi.position
                FROM page_items pi
                JOIN page_snapshots ps ON pi.page_snapshot_id = ps.id
                WHERE pi.item_type = 'singer' AND pi.item_id = (
                    SELECT id FROM singers WHERE name = ?
                )
                ORDER BY ps.crawled_at DESC
                LIMIT 20
            """, (singer_name,))
            stats['appearances'] = [dict(row) for row in cursor.fetchall()]
            
            return stats
    
    def get_top_appearing_songs(self, limit: int = 10) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT s.song_id, s.title, s.artist, COUNT(*) as appearance_count
                FROM page_items pi
                JOIN songs s ON pi.item_type = 'song' AND pi.item_id = s.id
                GROUP BY s.song_id
                ORDER BY appearance_count DESC
                LIMIT ?
            """, (limit,))
            return [dict(row) for row in cursor.fetchall()]
    
    def get_top_appearing_singers(self, limit: int = 10) -> List[dict]:
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                SELECT sg.name, COUNT(*) as appearance_count
                FROM page_items pi
                JOIN singers sg ON pi.item_type = 'singer' AND pi.item_id = sg.id
                GROUP BY sg.name
                ORDER BY appearance_count DESC
                LIMIT ?
            """, (limit,))
            return [dict(row) for row in cursor.fetchall()]
    
    def clear_rankings(self, ranking_type: str = None):
        with self.get_connection() as conn:
            cursor = conn.cursor()
            if ranking_type:
                cursor.execute("DELETE FROM rankings WHERE ranking_type = ?", (ranking_type,))
            else:
                cursor.execute("DELETE FROM rankings")
    
    def clear_search_keywords(self):
        with self.get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM search_keywords")


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="数据库管理工具")
    parser.add_argument("-d", "--db", type=str, default="gequke.db", help="数据库文件路径")
    parser.add_argument("--stats", action="store_true", help="显示统计信息")
    parser.add_argument("--clear-rankings", type=str, help="清除指定榜单数据")
    parser.add_argument("--clear-keywords", action="store_true", help="清除搜索关键词")
    
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
    
    if args.clear_rankings:
        db.clear_rankings(args.clear_rankings)
        print(f"已清除 {args.clear_rankings} 榜单数据")
    
    if args.clear_keywords:
        db.clear_search_keywords()
        print("已清除搜索关键词数据")


if __name__ == "__main__":
    main()
