# 歌曲客爬虫项目 - 任务总结文档

> 本文档记录项目当前状态、现有功能和下一步任务

## 📁 项目结构

```
gequke-downloader/
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

## 💻 现有功能命令

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

#### 搜索爬取 (search_crawler.py) ⭐ 新增
```powershell
# 搜索歌曲
uv run search_crawler.py "清明上"

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

## 🎯 "gequ" 命令行工具设计方案

### 设计目标

将现有的多个脚本整合为一个统一的命令行工具 `gequ`，提供更友好的用户体验。

### 命令结构

```
gequ <command> [subcommand] [options]

Commands:
  crawl      爬取数据
  download   下载歌曲
  db         数据库操作
  stats      统计查询
  search     搜索查询（新增）
  config     配置管理（新增）
```

### 详细命令设计

#### 1. `gequ crawl` - 爬取数据

```powershell
# 爬取主页
gequ crawl homepage

# 爬取排行榜
gequ crawl ranking new                # 爬取新歌榜第1页
gequ crawl ranking singer -s 1 -e 10  # 爬取歌手榜第1-10页
gequ crawl ranking new -p 3           # 爬取新歌榜第3页

# 从本地HTML测试
gequ crawl homepage -f "downloads/homepage.html"
gequ crawl ranking new -f "downloads/新歌榜.html"

# 保存到数据库（默认行为）
gequ crawl homepage
gequ crawl ranking new --save-db      # 显式指定

# 保存为JSON
gequ crawl homepage -o "output.json"
gequ crawl ranking new -o "ranking.json"

# 同时保存到数据库和JSON
gequ crawl homepage --save-db -o "homepage.json"
```

**选项**：
- `-f, --file` - 从本地HTML文件读取
- `-o, --output` - 输出JSON文件路径
- `-p, --page` - 指定页码
- `-s, --start-page` - 起始页码
- `-e, --end-page` - 结束页码
- `--save-db` - 保存到数据库（默认行为）
- `--no-db` - 不保存到数据库

#### 2. `gequ download` - 下载歌曲

```powershell
# 下载单首歌曲
gequ download 5863335

# 下载多首歌曲
gequ download 5863335 5863376 5863857

# 从文件读取歌曲ID列表
gequ download --from-file "song_ids.txt"

# 使用Cookie（解决403问题）
gequ download 5863335 --cookie "cookie_string"

# 指定输出目录
gequ download 5863335 -o "downloads"

# 只下载MP3（不嵌入封面）
gequ download 5863335 --no-cover

# 下载并保存记录到数据库
gequ download 5863335 --save-db
```

**选项**：
- `--cookie, -c` - 浏览器Cookie字符串
- `-o, --output` - 输出目录
- `--from-file` - 从文件读取歌曲ID
- `--no-cover` - 不下载封面
- `--save-db` - 保存下载记录到数据库
- `--skip-existing` - 跳过已存在的文件

#### 3. `gequ db` - 数据库操作

```powershell
# 显示统计
gequ db stats

gequ db singer [名称]
- 不指定名称：列出所有歌手
- 指定名称：显示歌手详情和出现统计
gequ db singer           # 列出所有歌手
gequ db singer 周杰伦    # 查看周杰伦详情和统计
gequ db song [歌曲ID]
- 不指定ID：列出所有歌曲
- 指定ID：显示歌曲详情和出现记录
gequ db song           # 列出所有歌曲
gequ db song 8         # 查看歌曲ID为8的详情
gequ db download
- 查看下载历史记录（包含歌曲标题、歌手、文件大小）
gequ db download       # 显示最近20条
gequ db download -n 50 # 显示最近50条
gequ db ranking [榜单类型]
- 不指定类型：列出所有排行榜数据
- 指定类型：筛选特定榜单
gequ db ranking           # 所有榜单
gequ db ranking douyin    # 抖音榜
gequ db ranking singer    # 歌手榜
所有命令支持 -n/--number 参数控制显示数量。                       # 优化数据库
```

**选项**：
- `-f, --file` - 文件路径
- `-o, --output` - 输出路径
- `--format` - 导出格式（json/csv）

#### 4. `gequ stats` - 统计查询

```powershell
# 数据库总览
gequ stats

# 歌曲统计
gequ stats song 5863335               # 基本信息
gequ stats song 5863335 --history     # 包含历史记录
gequ stats song 5863335 --format json # JSON格式输出

# 歌手统计
gequ stats singer "周杰伦"
gequ stats singer "周杰伦" --history

# 热门排行
gequ stats top-songs                  # 前10首
gequ stats top-songs -n 20            # 前20首
gequ stats top-singers
gequ stats top-singers -n 20

# 页面历史
gequ stats history homepage           # 主页快照历史
gequ stats history ranking            # 排行榜快照历史
gequ stats history ranking new        # 新歌榜快照历史

# 时间范围查询
gequ stats top-songs --from 2026-01-01 --to 2026-12-31
gequ stats history homepage --last 7d # 最近7天
```

**选项**：
- `-n, --number` - 显示数量
- `--history` - 显示历史记录
- `--format` - 输出格式（table/json/csv）
- `--from, --to` - 时间范围
- `--last` - 最近N天（如 7d, 30d）

#### 5. `gequ search` - 搜索查询（新增）

```powershell
# 搜索歌曲
gequ search song "周杰伦"              # 按歌手搜索
gequ search song "稻香"                # 按歌名搜索
gequ search song "周杰伦 稻香"         # 组合搜索

# 搜索歌手
gequ search singer "周"                # 模糊搜索

# 搜索关键词
gequ search keyword "爱情"             # 搜索关键词记录

# 高级搜索
gequ search song --artist "周杰伦" --title "稻香"
gequ search song --in-ranking new     # 在新歌榜中的歌曲
gequ search singer --min-appearances 5 # 出现5次以上的歌手
```

**选项**：
- `--artist` - 歌手名
- `--title` - 歌曲名
- `--in-ranking` - 在指定榜单中
- `--min-appearances` - 最小出现次数
- `-n, --number` - 返回数量
- `--format` - 输出格式

#### 6. `gequ config` - 配置管理（新增）

```powershell
# 显示配置
gequ config show
gequ config list

# 设置配置
gequ config set cookie "your_cookie_string"
gequ config set db-path "data/gequke.db"
gequ config set download-dir "downloads"
gequ config set output-format json

# 获取配置
gequ config get cookie
gequ config get db-path

# 删除配置
gequ config unset cookie

# 重置配置
gequ config reset
```

**配置项**：
- `cookie` - 浏览器Cookie
- `db-path` - 数据库路径（默认：gequke.db）
- `download-dir` - 下载目录（默认：downloads）
- `output-format` - 默认输出格式（json/table）
- `user-agent` - 自定义User-Agent
- `timeout` - 请求超时时间

**配置文件位置**：
- Windows: `%APPDATA%/gequ/config.json`
- Linux/Mac: `~/.config/gequ/config.json`

### 命令层次结构

```
gequ
├── crawl          爬取数据
│   ├── homepage   爬取主页
│   └── ranking    爬取排行榜
├── download       下载歌曲
├── db             数据库操作
│   ├── stats      显示统计
│   ├── save       保存数据
│   ├── clear      清除数据
│   ├── export     导出数据
│   ├── import     导入数据
│   ├── backup     备份数据库
│   └── restore    恢复数据库
├── stats          统计查询
│   ├── song       歌曲统计
│   ├── singer     歌手统计
│   ├── top-songs  热门歌曲
│   ├── top-singers 热门歌手
│   └── history    页面历史
├── search         搜索查询
│   ├── song       搜索歌曲
│   ├── singer     搜索歌手
│   └── keyword    搜索关键词
└── config         配置管理
    ├── show       显示配置
    ├── set        设置配置
    ├── get        获取配置
    ├── unset      删除配置
    └── reset      重置配置
```

### 使用示例

```powershell
# 1. 爬取主页并保存到数据库
gequ crawl homepage

# 2. 爬取新歌榜前3页
gequ crawl ranking new -s 1 -e 3

# 3. 查看数据库统计
gequ stats

# 4. 下载歌曲
gequ download 5863335

# 5. 查询歌曲出现统计
gequ stats song 5863335 --history

# 6. 查看热门歌手
gequ stats top-singers -n 20

# 7. 搜索周杰伦的歌曲
gequ search song "周杰伦"

# 8. 设置Cookie
gequ config set cookie "your_cookie_string"

# 9. 备份数据库
gequ db backup -o "backup_$(date +%Y%m%d).db"

# 10. 导出歌手数据
gequ db export singers -o "singers.csv" --format csv
```

## 🔧 技术实现要点

### 1. 命令行框架选择

推荐使用 **Click** 或 **Typer**（基于Click，更现代）

**Typer 示例**：
```python
import typer

app = typer.Typer()

@app.command()
def crawl(
    target: str = typer.Argument(..., help="homepage or ranking"),
    ranking_type: str = typer.Option(None, help="ranking type"),
    page: int = typer.Option(1, "-p", "--page"),
    save_db: bool = typer.Option(True, "--save-db/--no-db"),
):
    """爬取数据"""
    pass

if __name__ == "__main__":
    app()
```

### 2. 配置文件管理

使用 JSON 格式配置文件：
```json
{
  "cookie": "",
  "db_path": "gequke.db",
  "download_dir": "downloads",
  "output_format": "table",
  "user_agent": "Mozilla/5.0...",
  "timeout": 30
}
```

### 3. 数据库连接池

使用上下文管理器管理数据库连接：
```python
from contextlib import contextmanager

@contextmanager
def get_db():
    db = Database()
    try:
        yield db
    finally:
        pass  # Database handles cleanup
```

### 4. 输出格式化

支持多种输出格式：
- **table** - 表格形式（默认）
- **json** - JSON格式
- **csv** - CSV格式

使用 `rich` 或 `tabulate` 库实现美观的表格输出。

### 5. 进度显示

使用 `rich.progress` 或 `tqdm` 显示进度条：
```python
from rich.progress import Progress

with Progress() as progress:
    task = progress.add_task("爬取中...", total=100)
    # ... 更新进度
    progress.update(task, advance=10)
```

## 📦 依赖包

```
# 现有依赖
httpx>=0.27.0       # 异步HTTP客户端（替代requests）
beautifulsoup4>=4.12.0
lxml>=5.0.0
mutagen>=1.47.0

# CLI工具依赖
typer>=0.12.0       # 命令行框架
rich>=13.0.0        # 美化输出
```

## 🚀 已实现功能

### 阶段1：基础框架搭建 ✅
- [x] 创建 `src/gequ/` 包结构
- [x] 使用 httpx 实现异步爬虫模块
- [x] 实现 `gequ crawl` 子命令（homepage, ranking）
- [x] 实现 `gequ download` 子命令
- [x] 实现 `gequ db stats` 子命令
- [x] 实现 `gequ search` 命令
- [x] 实现 `gequ config` 子命令

### 阶段2：配置管理 ✅
- [x] 实现 `gequ config` 子命令
- [x] 创建配置文件管理模块（JSON格式）
- [x] 配置文件位置：
  - Windows: `%APPDATA%/gequ/config.json`
  - Linux/Mac: `~/.config/gequ/config.json`

## 🎯 下一步任务

### 阶段3：gequ search 得到的数据保存到数据库SQLite ✅
- [x] 实现 `gequ search` 除了爬取给定关键词歌曲后，将得到的信息保存到SQLite
- [x] 添加 `--no-db` 选项控制是否保存到数据库
- [x] 添加 `-f/--file` 选项支持从本地HTML文件读取

### 阶段4：统计查询功能 ✅
- [x] 实现 `gequ stats` 子命令完整功能
- [x] 支持 `gequ stats song` 和 `gequ stats singer`
- [x] 支持 `gequ stats top-songs` 和 `gequ stats top-singers`
- [x] 支持 `gequ stats history` 查看页面快照历史

### 阶段5：数据库增强功能
- [ ] 实现 `gequ db export/import` 数据导入导出
- [ ] 实现 `gequ db backup/restore` 备份恢复功能

### 阶段6：优化与完善
- [ ] 添加命令补全功能
- [ ] 添加详细的帮助文档
- [ ] 编写单元测试
- [ ] 打包发布到 PyPI

## 📝 注意事项

1. **Cookie 管理**
   - 网站可能需要Cookie才能访问
   - 建议通过配置文件持久化Cookie
   - 支持从浏览器自动导入Cookie

2. **错误处理**
   - 网络请求失败重试
   - 数据库操作异常处理
   - 友好的错误提示信息

3. **性能优化**
   - 批量插入数据
   - 异步爬取（可选）
   - 数据库查询优化

4. **用户体验**
   - 彩色输出
   - 进度显示
   - 命令补全
   - 详细的错误提示

## 🎯 项目目标

创建一个功能完善、易于使用的命令行工具，集成以下功能：
- ✅ 主页数据爬取
- ✅ 排行榜数据爬取（7种榜单）
- ✅ 关键词搜索功能
- ✅ 歌曲下载（MP3+歌词+封面）
- ✅ 数据库存储（7个表）
- ✅ 页面快照功能（支持复现和统计）
- ✅ 统计分析（歌曲/歌手出现次数）
- ✅ 统一命令行工具（gequ）✅ 已完成

---

**最后更新**: 2026-05-08
**状态**: CLI工具已实现核心功能，使用httpx异步HTTP客户端，typer命令行框架。阶段3、阶段4已完成。
