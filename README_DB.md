# SQLite 数据库设计方案总结

## 一、数据结构分析与复用

### 三个爬虫的重复数据

| 数据类型 | 出现位置 | 复用方式 |
|---------|---------|---------|
| **歌手信息** | homepage(热门歌手) + ranking(歌手榜) + song(artist字段) | 统一到 `singers` 表 |
| **歌曲信息** | ranking(歌曲榜) + download(歌曲详情) | 统一到 `songs` 表 |
| **排名数据** | ranking(各榜单) + homepage(热门排行) | 统一到 `rankings` 表 |

### 核心实体（3个）

1. **Singer（歌手）** - 在3个爬虫中都出现
2. **Song（歌曲）** - 在2个爬虫中都出现  
3. **RankingItem（排行项）** - 在2个爬虫中都出现

### 辅助实体（2个）

4. **SearchKeyword（搜索关键词）** - 仅 homepage
5. **DownloadRecord（下载记录）** - 仅 download

## 二、数据库表设计（5个表）

```sql
-- 1. singers - 歌手信息（核心表）
CREATE TABLE singers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,      -- 唯一约束：防止重复歌手
    avatar_url TEXT,
    songs_url TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- 2. songs - 歌曲信息（核心表）
CREATE TABLE songs (
    id INTEGER PRIMARY KEY,
    song_id INTEGER NOT NULL UNIQUE, -- 唯一约束：网站歌曲ID唯一
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    cover_url TEXT,
    mp3_url TEXT,
    play_id TEXT,
    lrc TEXT,
    extra_url TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- 3. rankings - 排行榜数据（关联表）
CREATE TABLE rankings (
    id INTEGER PRIMARY KEY,
    ranking_type TEXT NOT NULL,     -- singer/surge/new/douyin等
    rank INTEGER NOT NULL,
    song_id INTEGER,                -- 关联songs表
    singer_id INTEGER,              -- 关联singers表
    page INTEGER DEFAULT 1,
    crawled_at TIMESTAMP,
    UNIQUE(ranking_type, rank, crawled_at),  -- 唯一约束：防止重复排名
    FOREIGN KEY (song_id) REFERENCES songs(id),
    FOREIGN KEY (singer_id) REFERENCES singers(id)
);

-- 4. search_keywords - 搜索关键词
CREATE TABLE search_keywords (
    id INTEGER PRIMARY KEY,
    keyword TEXT NOT NULL,
    source TEXT NOT NULL,           -- latest/hot
    rank INTEGER,
    crawled_at TIMESTAMP,
    UNIQUE(keyword, source, crawled_at)
);

-- 5. downloads - 下载记录
CREATE TABLE downloads (
    id INTEGER PRIMARY KEY,
    song_id INTEGER NOT NULL,       -- 关联songs表
    file_path TEXT NOT NULL,
    file_size INTEGER,
    downloaded_at TIMESTAMP,
    FOREIGN KEY (song_id) REFERENCES songs(id)
);
```

## 三、解决重复数据的策略

### 策略1：UNIQUE 约束（表级）

```sql
-- 歌手名唯一：同一歌手只存一次
name TEXT NOT NULL UNIQUE

-- 歌曲ID唯一：同一歌曲只存一次
song_id INTEGER NOT NULL UNIQUE

-- 排名唯一：同一天同一榜单同一排名只存一次
UNIQUE(ranking_type, rank, crawled_at)
```

### 策略2：INSERT OR REPLACE（插入级）

```sql
INSERT INTO singers (name, avatar_url, songs_url)
VALUES (?, ?, ?)
ON CONFLICT(name) DO UPDATE SET      -- 冲突时更新
    avatar_url = excluded.avatar_url,
    songs_url = excluded.songs_url,
    updated_at = CURRENT_TIMESTAMP;
```

**优点**：
- 自动处理重复，无需先查询
- 更新时间戳，记录最后修改时间

### 策略3：COALESCE 保留旧数据（更新级）

```sql
INSERT INTO songs (song_id, title, artist, cover_url, mp3_url)
VALUES (?, ?, ?, ?, ?)
ON CONFLICT(song_id) DO UPDATE SET
    cover_url = COALESCE(excluded.cover_url, cover_url),  -- 新数据优先，无则保留旧数据
    mp3_url = COALESCE(excluded.mp3_url, mp3_url),
    updated_at = CURRENT_TIMESTAMP;
```

**优点**：
- 逐步补充信息（第一次只有title，第二次补充cover_url）
- 不会覆盖已有的完整数据

## 四、使用流程

### 保存主页数据

```powershell
uv run save_to_db.py --homepage
# 或从本地HTML测试
uv run save_to_db.py --homepage-file "downloads/homepage.html"
```

### 保存排行榜数据

```powershell
# 保存新歌榜第1页
uv run save_to_db.py --ranking new

# 保存歌手榜第1-10页
uv run save_to_db.py --ranking singer --start-page 1 --end-page 10

# 或从本地HTML测试
uv run save_to_db.py --ranking new --ranking-file "downloads/新歌榜.html"
```

### 查看统计信息

```powershell
uv run save_to_db.py --stats
```

### 数据库管理

```powershell
# 查看统计
uv run database.py --stats

# 清除新歌榜数据
uv run database.py --clear-rankings new

# 清除搜索关键词
uv run database.py --clear-keywords
```

## 五、数据关系图

```
┌─────────────┐
│  singers    │
│  (歌手)     │
└──────┬──────┘
       │ 1
       │
       │ N (歌手榜)
┌──────┴──────┐       N ┌─────────────┐
│  rankings   │─────────│  songs      │
│  (排行榜)   │         │  (歌曲)     │
└──────┬──────┘         └──────┬──────┘
       │                       │ 1
       │                       │
       │                       │ N
┌──────┴──────┐         ┌──────┴──────┐
│search_keywords│         │ downloads  │
│  (关键词)    │         │  (下载)    │
└─────────────┘         └─────────────┘
```

**关系说明**：
- singers ↔ rankings：歌手榜排名关联歌手
- songs ↔ rankings：歌曲榜排名关联歌曲
- songs ↔ downloads：下载记录关联歌曲

## 六、核心设计思想

### 1. 数据去重（自动）
- UNIQUE约束 + ON CONFLICT自动处理
- 无需手动判断是否存在

### 2. 数据补充（渐进）
- COALESCE保留旧数据，新数据优先
- 每次爬取补充缺失字段

### 3. 数据关联（外键）
- rankings关联singers/songs
- downloads关联songs
- 查询时JOIN获取完整信息

### 4. 时间记录（时间戳）
- created_at：创建时间
- updated_at：更新时间
- crawled_at：爬取时间（用于区分历史排名）

## 七、已创建文件

1. **models.py** - 统一数据模型（5个实体类）
2. **database.py** - SQLite数据库管理（5个表 + 所有操作）
3. **save_to_db.py** - 数据保存示例（连接爬虫与数据库）
4. **DATABASE_DESIGN.md** - 详细设计文档

所有文件已测试通过！