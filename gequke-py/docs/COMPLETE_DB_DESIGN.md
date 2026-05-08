# 数据库完整设计方案（含页面快照）

## 一、完整表结构（7个表）

### 核心实体表（2个）

#### 1. singers - 歌手信息
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

#### 2. songs - 歌曲信息
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

### 排行记录表（1个）

#### 3. rankings - 排行榜数据
```sql
CREATE TABLE rankings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ranking_type TEXT NOT NULL,         -- 榜单类型（singer/surge/new等）
    rank INTEGER NOT NULL,              -- 排名
    song_id INTEGER,                    -- 关联歌曲ID
    singer_id INTEGER,                  -- 关联歌手ID
    page INTEGER DEFAULT 1,             -- 页码
    crawled_at TIMESTAMP,
    UNIQUE(ranking_type, rank, crawled_at),
    FOREIGN KEY (song_id) REFERENCES songs(id),
    FOREIGN KEY (singer_id) REFERENCES singers(id)
);
```

### 页面快照表（2个）⭐ 新增

#### 4. page_snapshots - 页面快照
```sql
CREATE TABLE page_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_type TEXT NOT NULL,           -- homepage / ranking
    ranking_type TEXT,                  -- 如果是排行榜页
    page_number INTEGER DEFAULT 1,      -- 页码
    url TEXT,                           -- 页面URL
    title TEXT,                         -- 页面标题
    crawled_at TIMESTAMP,
    UNIQUE(page_type, ranking_type, page_number, crawled_at)
);
```

#### 5. page_items - 页面条目
```sql
CREATE TABLE page_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_snapshot_id INTEGER NOT NULL, -- 关联页面快照
    item_type TEXT NOT NULL,            -- song / singer / keyword
    item_id INTEGER,                    -- 关联singers.id或songs.id
    position INTEGER DEFAULT 0,         -- 在页面中的位置/排名
    extra_data TEXT,                    -- JSON格式的额外数据
    FOREIGN KEY (page_snapshot_id) REFERENCES page_snapshots(id)
);
```

### 辅助表（2个）

#### 6. search_keywords - 搜索关键词
```sql
CREATE TABLE search_keywords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    keyword TEXT NOT NULL,
    source TEXT NOT NULL,               -- latest/hot
    rank INTEGER,
    crawled_at TIMESTAMP,
    UNIQUE(keyword, source, crawled_at)
);
```

#### 7. downloads - 下载记录
```sql
CREATE TABLE downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER,
    downloaded_at TIMESTAMP,
    FOREIGN KEY (song_id) REFERENCES songs(id)
);
```

## 二、数据关系图

```
┌─────────────┐
│  singers    │
│  (歌手)     │
└──────┬──────┘
       │
       ├────── N ──────┐
       │               │
       │               ▼
┌──────┴──────┐ ┌─────────────┐
│ page_items  │ │  songs      │
│  (条目)     │ │  (歌曲)     │
└──────┬──────┘ └──────┬──────┘
       │               │
       │ N             │ N
       │               │
┌──────┴──────┐ ┌──────┴──────┐
│page_snapshots│ │ downloads   │
│  (快照)     │ │  (下载)     │
└──────┬──────┘ └─────────────┘
       │
       │
       ▼
┌─────────────┐
│  rankings   │
│  (排行)     │
└─────────────┘
```

## 三、解决重复数据的完整策略

### 1. UNIQUE 约束（自动去重）

| 表 | 唯一约束 | 说明 |
|----|---------|------|
| singers | name | 歌手名唯一 |
| songs | song_id | 歌曲ID唯一 |
| rankings | (type, rank, crawled_at) | 同一排名唯一 |
| page_snapshots | (type, ranking_type, page, crawled_at) | 同一页面唯一 |

### 2. ON CONFLICT DO UPDATE（自动更新）

```sql
INSERT INTO singers (name, avatar_url, songs_url)
VALUES (?, ?, ?)
ON CONFLICT(name) DO UPDATE SET
    avatar_url = excluded.avatar_url,
    songs_url = excluded.songs_url,
    updated_at = CURRENT_TIMESTAMP;
```

### 3. COALESCE 保留旧数据（渐进补充）

```sql
INSERT INTO songs (song_id, title, artist, cover_url, mp3_url)
VALUES (?, ?, ?, ?, ?)
ON CONFLICT(song_id) DO UPDATE SET
    cover_url = COALESCE(excluded.cover_url, cover_url),
    mp3_url = COALESCE(excluded.mp3_url, mp3_url);
```

## 四、新增的查询统计功能

### 1. 歌曲出现统计
```powershell
uv run save_to_db.py --song-stats 5863335

输出：
  总出现次数: 1
  主页出现: 0 次
  排行榜出现: 1 次
  最近出现记录: ranking | new | 第1页 | 排名1
```

### 2. 歌手出现统计
```powershell
uv run save_to_db.py --singer-stats "周杰伦"

输出：
  总出现次数: 2
  主页出现: 1 次
  排行榜出现: 1 次
  最近出现记录: ranking | singer | 第1页 | 排名1
```

### 3. 热门歌曲排行
```powershell
uv run save_to_db.py --top-songs

输出：
  1. 自生光 - 袁一琦 (出现 1 次)
  2. Someone to Love - 严浩翔 (出现 1 次)
  ...
```

### 4. 热门歌手排行
```powershell
uv run save_to_db.py --top-singers

输出：
  1. 林俊杰 (出现 2 次)
  2. 周杰伦 (出现 2 次)
  ...
```

### 5. 页面快照历史
```powershell
uv run save_to_db.py --page-history homepage
uv run save_to_db.py --page-history ranking

输出：
  ID:1 | 2026-05-07 12:08:47 | - | 第1页 | 歌曲客主页
  ID:2 | 2026-05-07 12:08:54 | new | 第1页 | 新歌榜
  ID:3 | 2026-05-07 12:09:01 | singer | 第1页 | 热门歌手排行
```

## 五、实际应用场景

### 场景1: 歌曲热度分析
```python
# 每天定时爬取排行榜
# 统计歌曲出现次数变化
stats = db.get_song_appearance_stats(song_id)
if stats['ranking_count'] > 3:
    print("这首歌持续热门！")
```

### 场景2: 歌手影响力分析
```python
# 统计歌手在不同榜单的出现情况
stats = db.get_singer_appearance_stats("周杰伦")
total = stats['homepage_count'] + stats['ranking_count']
print(f"周杰伦影响力指数: {total}")
```

### 场景3: 榜单趋势分析
```python
# 查看某个榜单的历史快照
snapshots = db.get_page_snapshots(page_type="ranking", limit=30)
# 分析榜单变化趋势
```

### 场景4: 页面内容复现
```python
# 获取某个快照的所有条目
items = db.get_page_items_by_snapshot(snapshot_id)
# 按照position排序，复现页面内容
for item in items:
    print(f"{item['position']}. {item['title']} - {item['artist']}")
```

## 六、文件列表

| 文件 | 说明 |
|------|------|
| **models.py** | 数据模型（7个实体类） |
| **database.py** | 数据库管理（7个表 + 所有操作） |
| **save_to_db.py** | 数据保存脚本（含统计查询） |
| **DATABASE_DESIGN.md** | 原始设计文档 |
| **PAGE_SNAPSHOT_GUIDE.md** | 页面快照使用指南 |
| **COMPLETE_DB_DESIGN.md** | 完整设计文档（本文档） |

## 七、完整命令列表

### 数据保存
```powershell
uv run save_to_db.py --homepage
uv run save_to_db.py --homepage-file "downloads/homepage.html"
uv run save_to_db.py --ranking new -s 1 -e 5
uv run save_to_db.py --ranking singer --ranking-file "downloads/singer.html"
```

### 统计查询
```powershell
uv run save_to_db.py --stats
uv run save_to_db.py --song-stats 5863335
uv run save_to_db.py --singer-stats "周杰伦"
uv run save_to_db.py --top-songs
uv run save_to_db.py --top-singers
uv run save_to_db.py --page-history homepage
uv run save_to_db.py --page-history ranking
```

### 数据库管理
```powershell
uv run database.py --stats
uv run database.py --clear-rankings new
uv run database.py --clear-keywords
```

## 八、测试结果

✅ 所有功能已测试通过：

```
数据库统计:
  歌手数: 18
  歌曲数: 10
  排行记录数: 40
  搜索关键词数: 40
  下载记录数: 0
  页面快照数: 3      ⭐ 新增
  页面条目数: 40     ⭐ 新增
```

歌曲出现统计：✅ 通过  
歌手出现统计：✅ 通过  
热门歌曲排行：✅ 通过  
热门歌手排行：✅ 通过  
页面快照历史：✅ 通过

## 九、扩展性设计

数据库设计考虑了未来扩展：

1. **添加新榜单** - 只需在 rankings.ranking_type 添加新值
2. **添加新页面类型** - 在 page_snapshots.page_type 添加新值
3. **添加新条目类型** - 在 page_items.item_type 添加新值
4. **存储额外数据** - 使用 extra_data 字段（JSON格式）
5. **追踪变化** - 通过时间戳查询历史记录

完美支持未来的功能扩展！
