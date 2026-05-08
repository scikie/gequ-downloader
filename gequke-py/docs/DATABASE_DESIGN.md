# SQLite 数据库设计说明

## 数据结构分析

### 三个爬虫的数据结构对比

| 爬虫 | 数据项 | 重复数据 |
|------|--------|----------|
| homepage_crawler | SearchKeyword, RankedKeyword, HotSinger | 歌手名、歌曲关键词 |
| ranking_crawler | SongRank, SingerRank | 歌手、歌曲信息 |
| download_crawler | song_info | 歌曲详细信息 |

### 可复用的核心实体

1. **歌手 (Singer)** - 在多处出现
   - homepage: HotSinger
   - ranking: SingerRank
   - song: artist字段

2. **歌曲 (Song)** - 在多处出现
   - ranking: SongRank
   - download: song_info

## 数据库表设计

### 1. singers 表 - 歌手信息
```sql
CREATE TABLE singers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,          -- 歌手名（唯一）
    avatar_url TEXT,                    -- 头像URL
    songs_url TEXT,                     -- 歌曲列表URL
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);
```

### 2. songs 表 - 歌曲信息
```sql
CREATE TABLE songs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id INTEGER NOT NULL UNIQUE,    -- 网站歌曲ID（唯一）
    title TEXT NOT NULL,                -- 歌曲名
    artist TEXT NOT NULL,               -- 歌手名
    cover_url TEXT,                     -- 封面URL
    mp3_url TEXT,                       -- MP3下载URL
    play_id TEXT,                       -- 播放ID
    lrc TEXT,                           -- 歌词
    extra_url TEXT,                     -- 备用下载链接
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);
```

### 3. rankings 表 - 排行榜数据
```sql
CREATE TABLE rankings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ranking_type TEXT NOT NULL,         -- 榜单类型（singer/surge/new等）
    rank INTEGER NOT NULL,              -- 排名
    song_id INTEGER,                    -- 关联歌曲ID
    singer_id INTEGER,                  -- 关联歌手ID
    page INTEGER DEFAULT 1,             -- 页码
    crawled_at TIMESTAMP,               -- 爬取时间
    UNIQUE(ranking_type, rank, crawled_at),
    FOREIGN KEY (song_id) REFERENCES songs(id),
    FOREIGN KEY (singer_id) REFERENCES singers(id)
);
```

### 4. search_keywords 表 - 搜索关键词
```sql
CREATE TABLE search_keywords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    keyword TEXT NOT NULL,              -- 关键词
    source TEXT NOT NULL,               -- 来源（latest/hot）
    rank INTEGER,                       -- 排名（可选）
    crawled_at TIMESTAMP,
    UNIQUE(keyword, source, crawled_at)
);
```

### 5. downloads 表 - 下载记录
```sql
CREATE TABLE downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id INTEGER NOT NULL,          -- 关联歌曲ID
    file_path TEXT NOT NULL,           -- 文件路径
    file_size INTEGER,                  -- 文件大小
    downloaded_at TIMESTAMP,
    FOREIGN KEY (song_id) REFERENCES songs(id)
);
```

## 解决重复数据问题

### 策略1: 使用 UNIQUE 约束
- singers.name - 歌手名唯一
- songs.song_id - 歌曲ID唯一
- rankings(ranking_type, rank, crawled_at) - 同一天的同一排名唯一

### 策略2: 使用 INSERT OR REPLACE / ON CONFLICT
```sql
INSERT INTO singers (name, avatar_url, songs_url)
VALUES (?, ?, ?)
ON CONFLICT(name) DO UPDATE SET
    avatar_url = excluded.avatar_url,
    songs_url = excluded.songs_url,
    updated_at = CURRENT_TIMESTAMP;
```

### 策略3: 使用 COALESCE 保留旧数据
```sql
INSERT INTO songs (song_id, title, artist, cover_url, mp3_url)
VALUES (?, ?, ?, ?, ?)
ON CONFLICT(song_id) DO UPDATE SET
    cover_url = COALESCE(excluded.cover_url, cover_url),
    mp3_url = COALESCE(excluded.mp3_url, mp3_url);
```

## 使用示例

### 保存歌手榜数据
```python
from database import Database
from models import Singer, RankingItem

db = Database()

# 保存歌手信息
singer = Singer(name="周杰伦", avatar_url="...", songs_url="/ss/周杰伦")
db.insert_singer(singer)

# 保存排行榜项
ranking = RankingItem(
    ranking_type="singer",
    rank=1,
    item_name="周杰伦",
    item_type="singer"
)
db.insert_ranking_item(ranking)
```

### 保存歌曲榜数据
```python
from models import Song, RankingItem

# 保存歌曲信息
song = Song(
    song_id=5863335,
    title="不跪的花",
    artist="央金拉姆",
    cover_url="..."
)
db.insert_song(song)

# 保存排行榜项
ranking = RankingItem(
    ranking_type="new",
    rank=1,
    item_id=5863335,
    item_type="song"
)
db.insert_ranking_item(ranking)
```

### 查询数据
```python
# 查询新歌榜前10名
new_songs = db.get_ranking_by_type("new", limit=10)

# 查询歌手信息
singer = db.get_singer_by_name("周杰伦")

# 查看统计信息
stats = db.get_stats()
```

## 数据库管理命令

```powershell
# 显示统计信息
uv run database.py --stats

# 清除新歌榜数据
uv run database.py --clear-rankings new

# 清除搜索关键词
uv run database.py --clear-keywords
```
