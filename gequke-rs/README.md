# Gequ CLI - Rust 版本

歌曲客网站爬虫工具的 Rust 实现。

## 功能

- 爬取主页数据
- 爬取排行榜数据（7种榜单）
- 搜索歌曲
- 下载歌曲（MP3 + 歌词）
- 数据库管理
- 统计查询

## 安装

### 编译

```powershell
cd gequke-rs
cargo build --release
```

编译后的可执行文件位于 `target/release/gequ.exe`

### 安装到系统

```powershell
cargo install --path .
```

## 使用

### 基本命令

```powershell
gequ <command> [subcommand] [options]
```

### 命令列表

#### 爬取数据

```powershell
# 爬取主页
gequ crawl homepage
gequ crawl homepage -o output.json       # 保存为JSON
gequ crawl homepage --no-db              # 不保存到数据库

# 爬取排行榜
gequ crawl ranking new                   # 爬取新歌榜第1页
gequ crawl ranking singer -s 1 -e 10     # 爬取歌手榜第1-10页
gequ crawl ranking new -p 3              # 爬取新歌榜第3页

# 支持的榜单类型：
# singer - 歌手榜
# surge - 飙升榜
# new - 新歌榜
# douyin - 抖音榜
# jingdian - 怀旧榜
# dianyin - 电音榜
# wwdj - DJ榜
```

#### 下载歌曲

```powershell
# 下载单首歌曲
gequ download song 5863335
gequ download song 5863335 -o downloads   # 指定输出目录
gequ download song 5863335 --no-cover     # 不嵌入封面

# 下载多首歌曲
gequ download songs 5863335 5863376 5863857
```

#### 搜索歌曲

```powershell
gequ search "周杰伦"
gequ search "清明上" -o output.json      # 保存为JSON
gequ search "周杰伦" --no-db              # 不保存到数据库
```

#### 数据库操作

```powershell
# 显示统计
gequ db stats

# 查询歌手
gequ db singer             # 列出所有歌手（默认20条）
gequ db singer 周杰伦      # 查看歌手详情
gequ db singer -n 50       # 显示50条记录

# 查询歌曲
gequ db song               # 列出所有歌曲
gequ db song 5863335       # 查看歌曲详情

# 查询下载历史
gequ db download

# 查询排行榜
gequ db ranking            # 所有排行榜数据
gequ db ranking new        # 新歌榜数据
```

#### 统计查询

```powershell
# 歌曲统计
gequ stats song 5863335

# 歌手统计
gequ stats singer 周杰伦

# 热门歌曲排行
gequ stats top-songs       # 前10首
gequ stats top-songs -n 20
gequ stats top-songs --last 7d     # 最近7天
gequ stats top-songs --from 2026-01-01 --to 2026-12-31

# 热门歌手排行
gequ stats top-singers

# 页面快照历史
gequ stats history homepage
gequ stats history ranking
gequ stats history search
```

#### 配置管理

```powershell
# 显示配置
gequ config show

# 设置配置
gequ config set cookie "your_cookie_string"
gequ config set db_path "data/gequke.db"
gequ config set download_dir "downloads"
gequ config set timeout 30

# 获取配置
gequ config get cookie

# 重置配置
gequ config reset
```

## 配置文件

配置文件位置：
- Windows: `%APPDATA%/gequ/config.json`
- Linux/Mac: `~/.config/gequ/config.json`

配置项：
- `cookie` - 网站Cookie（用于需要登录的功能）
- `db_path` - 数据库路径（默认：gequke.db）
- `download_dir` - 下载目录（默认：downloads）
- `output_format` - 输出格式（默认：table）
- `user_agent` - User-Agent（默认：浏览器UA）
- `timeout` - 请求超时时间（默认：30秒）

## 数据库结构

工具使用 SQLite 数据库存储数据，包含以下表：

1. **singers** - 歌手信息
2. **songs** - 歌曲信息
3. **rankings** - 排行榜数据
4. **search_keywords** - 搜索关键词
5. **downloads** - 下载记录
6. **page_snapshots** - 页面快照
7. **page_items** - 页面条目

## 技术栈

- **语言**: Rust
- **异步运行时**: Tokio
- **HTTP客户端**: Reqwest
- **HTML解析**: Scraper
- **数据库**: Rusqlite (SQLite)
- **CLI框架**: Clap
- **表格输出**: Tabled
- **配置管理**: Serde JSON
- **错误处理**: Anyhow

## 与 Python 版本的对比

Rust 版本相比 Python 版本具有以下优势：

1. **性能更快** - Rust 的编译优化带来更高的执行效率
2. **内存更安全** - Rust 的内存安全机制避免了常见错误
3. **单文件部署** - 编译后为单个可执行文件，无需依赖环境
4. **并发更好** - Tokio 异步运行时提供优秀的并发支持

## 注意事项

1. 部分功能需要设置 Cookie 才能正常工作（如下载功能）
2. 建议从浏览器复制 Cookie 字符串并使用 `gequ config set cookie` 设置
3. 数据库会在首次运行时自动创建

## 版本

当前版本：0.1.0