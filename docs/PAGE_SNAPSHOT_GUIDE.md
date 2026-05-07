# 页面快照功能说明

## 新增功能概述

为了支持复现页面和统计分析，新增了两个表：

1. **page_snapshots** - 页面快照表
2. **page_items** - 页面条目表

## 数据库表结构（新增2个表）

### 1. page_snapshots 表 - 页面快照
```sql
CREATE TABLE page_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_type TEXT NOT NULL,           -- homepage / ranking
    ranking_type TEXT,                  -- 如果是排行榜页，指定榜单类型
    page_number INTEGER DEFAULT 1,      -- 页码
    url TEXT,                           -- 页面URL
    title TEXT,                         -- 页面标题
    crawled_at TIMESTAMP,               -- 爬取时间
    UNIQUE(page_type, ranking_type, page_number, crawled_at)
);
```

**用途**：记录每次爬取的页面状态，用于：
- 复现历史页面
- 统计某个页面的爬取频率
- 追踪页面变化

### 2. page_items 表 - 页面条目
```sql
CREATE TABLE page_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_snapshot_id INTEGER NOT NULL,  -- 关联页面快照
    item_type TEXT NOT NULL,            -- song / singer / keyword
    item_id INTEGER,                    -- 关联singers.id或songs.id
    position INTEGER DEFAULT 0,         -- 在页面中的位置/排名
    extra_data TEXT,                    -- JSON格式的额外数据
    FOREIGN KEY (page_snapshot_id) REFERENCES page_snapshots(id)
);
```

**用途**：记录页面上出现的每个条目，用于：
- 统计歌曲/歌手在页面的出现次数
- 追踪排名变化
- 复现页面内容

## 使用示例

### 1. 保存数据（自动创建页面快照）

```powershell
# 保存主页数据
uv run save_to_db.py --homepage-file "downloads/homepage.html"

# 保存新歌榜数据
uv run save_to_db.py --ranking new --ranking-file "downloads/新歌榜.html"

# 保存歌手榜数据
uv run save_to_db.py --ranking singer --ranking-file "downloads/歌手榜.html"
```

### 2. 查看统计信息

```powershell
# 数据库统计
uv run save_to_db.py --stats

输出：
  歌手数: 18
  歌曲数: 10
  排行记录数: 40
  搜索关键词数: 40
  下载记录数: 0
  页面快照数: 3
  页面条目数: 40
```

### 3. 查询歌曲出现统计

```powershell
uv run save_to_db.py --song-stats 5863335

输出：
歌曲 5863335 统计:
  总出现次数: 1
  主页出现: 0 次
  排行榜出现: 1 次

  最近出现记录:
    - ranking | new | 第1页 | 排名1 | 2026-05-07 12:08:54
```

### 4. 查询歌手出现统计

```powershell
uv run save_to_db.py --singer-stats "周杰伦"

输出：
歌手 周杰伦 统计:
  总出现次数: 2
  主页出现: 1 次
  排行榜出现: 1 次

  最近出现记录:
    - ranking | singer | 第1页 | 排名1 | 2026-05-07 12:09:01
    - homepage | - | 第1页 | 排名2 | 2026-05-07 12:08:47
```

### 5. 查看出现次数最多的歌曲

```powershell
uv run save_to_db.py --top-songs

输出：
出现次数最多的歌曲:
  1. 自生光 - 袁一琦 (出现 1 次)
  2. Someone to Love - 严浩翔 (出现 1 次)
  ...
```

### 6. 查看出现次数最多的歌手

```powershell
uv run save_to_db.py --top-singers

输出：
出现次数最多的歌手:
  1. 林俊杰 (出现 2 次)
  2. 周杰伦 (出现 2 次)
  ...
```

### 7. 查看页面快照历史

```powershell
# 查看主页快照历史
uv run save_to_db.py --page-history homepage

输出：
homepage 页面快照历史:
  ID:1 | 2026-05-07 12:08:47 | - | 第1页 | 歌曲客主页

# 查看排行榜快照历史
uv run save_to_db.py --page-history ranking

输出：
ranking 页面快照历史:
  ID:3 | 2026-05-07 12:09:01 | singer | 第1页 | 热门歌手排行
  ID:2 | 2026-05-07 12:08:54 | new | 第1页 | 新歌榜
```

## 实际应用场景

### 场景1: 统计某首歌的热度趋势
```python
# 每天爬取排行榜，记录页面快照
# 统计某首歌在不同时间点的排名变化
stats = db.get_song_appearance_stats(song_id)
# 分析出现次数和排名变化
```

### 场景2: 找出最热门的歌手
```python
# 统计所有歌手在主页和排行榜的出现次数
top_singers = db.get_top_appearing_singers(limit=10)
# 林俊杰、周杰伦等多次出现的歌手热度最高
```

### 场景3: 复现历史页面
```python
# 获取某个时间点的页面快照
snapshot = db.get_page_snapshots(page_type="ranking", limit=1)
# 获取该页面的所有条目
items = db.get_page_items_by_snapshot(snapshot['id'])
# 复现该时间点的排行榜内容
```

### 场景4: 分析歌曲推广效果
```python
# 歌曲刚发布时：只在new榜单出现
# 歌曲爆火后：在多个榜单出现，甚至出现在主页
stats = db.get_song_appearance_stats(song_id)
# 根据出现频率和排名变化判断推广效果
```

## 数据流程

```
爬虫爬取页面
    ↓
创建页面快照 (page_snapshots)
    ↓
提取页面条目
    ↓
保存实体数据 (singers, songs)
    ↓
创建页面条目关联 (page_items)
    ↓
统计分析 (出现次数、排名变化)
```

## 数据库关系图（更新）

```
┌─────────────┐
│  singers    │
│  (歌手)     │
└──────┬──────┘
       │
       │ N
       │
┌──────┴──────┐       ┌─────────────┐
│ page_items  │───────│page_snapshots│
│  (条目)     │       │  (快照)     │
└──────┬──────┘       └──────┬──────┘
       │                     │
       │ N                   │ 1
       │                     │
┌──────┴──────┐       ┌──────┴──────┐
│  songs      │       │rankings     │
│  (歌曲)     │       │  (排行)     │
└─────────────┘       └─────────────┘
```

## 总结

新增的页面快照功能可以：
1. **记录历史** - 保存每次爬取的页面状态
2. **统计分析** - 统计歌曲/歌手的出现次数
3. **趋势分析** - 追踪排名变化趋势
4. **页面复现** - 复现任意时间点的页面内容

为后续的数据分析和可视化提供了基础！
