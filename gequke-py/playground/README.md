# playground 用于爬取歌曲的实验代码
## 📁 项目结构

```
playground/
├── models.py                    # 数据模型（7个实体类）
├── database.py                  # 数据库管理（7个表）
├── homepage_crawler.py          # 主页爬虫
├── ranking_crawler.py           # 排行榜爬虫
├── search_crawler.py            # 搜索爬虫
├── download_crawler.py          # 歌曲下载器
├── save_to_db.py                # 数据保存脚本（含统计查询）
├── gequke.db                    # SQLite数据库
├── downloads/                   # 下载目录
│   ├── homepage.json
│   ├── 新歌榜_page_1.json
│   ├── 热门歌手排行_page_1.json
│   └── 关键词-搜索结果.json
└── docs/
    ├── DATABASE_DESIGN.md       # 数据库设计文档
    ├── PAGE_SNAPSHOT_GUIDE.md   # 页面快照使用指南
    └── COMPLETE_DB_DESIGN.md    # 完整设计文档
```

## 🗄️ 数据库结构（7个表）

### 核心表
1. **singers** - 歌手信息（name唯一）
2. **songs** - 歌曲信息（song_id唯一）
3. **rankings** - 排行榜数据

### 页面快照表 ⭐
4. **page_snapshots** - 页面快照
5. **page_items** - 页面条目

### 辅助表
6. **search_keywords** - 搜索关键词
7. **downloads** - 下载记录

## 💻 功能命令

### 1️⃣ 爬取功能

#### 主页爬取 (homepage_crawler.py)
```powershell
# 从网站爬取主页
uv run homepage_crawler.py

# 从本地HTML测试
uv run homepage_crawler.py -f "downloads/homepage.html"

# 指定输出路径
uv run homepage_crawler.py -o "path/to/output.json"
```

**输出**：downloads/homepage.json

#### 排行榜爬取 (ranking_crawler.py)
```powershell
# 爬取新歌榜第1页
uv run ranking_crawler.py new -p 1

# 爬取歌手榜第1-10页
uv run ranking_crawler.py singer -s 1 -e 10

# 从本地HTML测试
uv run ranking_crawler.py -f "downloads/新歌榜.html" -p 1

# 支持的榜单类型：
# singer, surge, new, douyin, jingdian, dianyin, wwdj
```

**输出**：downloads/榜单名_page_X.json

#### 搜索爬取 (search_crawler.py)
```powershell
# 搜索歌曲
uv run search_crawler.py "清明"

# 从本地HTML测试
uv run search_crawler.py -f "downloads/搜索结果.html"

# 指定输出目录
uv run search_crawler.py "周杰伦" -o "downloads"
```

**输出**：downloads/关键词-搜索结果.json

#### 歌曲下载 (download_crawler.py)
```powershell
# 下载歌曲（通过song_id）
uv run download_crawler.py 5863335

# 使用Cookie（解决403问题）
uv run download_crawler.py 5863335 -c "cookie_string"

# 指定输出目录
uv run download_crawler.py 5863335 -o "downloads"
```

**输出**：MP3文件、歌词文件、封面图片

### 2️⃣ 数据库功能

#### 保存数据到数据库 (save_to_db.py)
```powershell
# 保存主页数据
uv run save_to_db.py --homepage
uv run save_to_db.py --homepage-file "downloads/homepage.html"

# 保存排行榜数据
uv run save_to_db.py --ranking new
uv run save_to_db.py --ranking singer -s 1 -e 10
uv run save_to_db.py --ranking new --ranking-file "downloads/新歌榜.html"

# 保存搜索数据
uv run save_to_db.py --search "清明上"
uv run save_to_db.py --search-file "downloads/搜索结果.html"

# 指定数据库路径
uv run save_to_db.py --stats -d "data/gequke.db"
```

#### 查询统计 (save_to_db.py)
```powershell
# 显示数据库统计
uv run save_to_db.py --stats

# 查询歌曲出现统计
uv run save_to_db.py --song-stats 5863335

# 查询歌手出现统计
uv run save_to_db.py --singer-stats "周杰伦"

# 查看热门歌曲（出现次数最多）
uv run save_to_db.py --top-songs

# 查看热门歌手
uv run save_to_db.py --top-singers

# 查看页面快照历史
uv run save_to_db.py --page-history homepage
uv run save_to_db.py --page-history ranking
```

#### 数据库管理 (database.py)
```powershell
# 显示统计
uv run database.py --stats

# 清除榜单数据
uv run database.py --clear-rankings new

# 清除搜索关键词
uv run database.py --clear-keywords
```