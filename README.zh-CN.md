<p align="center">
  <h1 align="center">🎵 歌曲客下载器 (Gequ Downloader)</h1>
  <p align="center">gequke.com 命令行爬虫、搜索与下载工具</p>
  <p align="center">
    <a href="#-功能特性">功能特性</a> •
    <a href="#-双版本实现">双版本实现</a> •
    <a href="#-快速开始">快速开始</a> •
    <a href="#-命令指南">命令指南</a> •
    <a href="#-数据库">数据库</a>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust"/>
    <img src="https://img.shields.io/badge/Python-3.10%2B-blue?logo=python"/>
    <img src="https://img.shields.io/badge/license-MIT-green"/>
  </p>
  <p align="center">
    <a href="README.md">English Version</a>
  </p>
</p>

---

## ✨ 功能特性

- **主页爬取** — 获取网站首页信息
- **排行榜爬取** — 支持 7 种榜单：歌手榜、飙升榜、新歌榜、抖音榜、怀旧榜、电音榜、DJ 榜
- **关键词搜索** — 按任意关键词搜索歌曲
- **歌曲下载** — 下载 MP3 音频 + 同步歌词 + 专辑封面，自动写入 ID3 标签
- **数据持久化** — SQLite 结构化存储
- **统计分析** — 歌曲/歌手出现次数统计、热门排行、时间范围过滤
- **页面快照** — 每次爬取存档，支持历史回溯与增量追踪
- **可配置** — JSON 配置文件，支持 Cookie、下载目录、超时等自定义

## 🦀 🐍 双版本实现

提供 **Rust** 和 **Python** 两个版本，按需选择：

| 对比项 | Rust (`gequke-rs/`) | Python (`gequke-py/`) |
|--------|---------------------|-----------------------|
| **运行方式** | 编译为原生二进制 | 需 Python 3.10+ 解释器 |
| **性能** | 🚀 极速 | ⚡ 够快 |
| **部署** | 单文件 (~10 MB)，零依赖 | 需 pip/uv 安装 |
| **并发** | Tokio 异步运行时 | httpx 异步 |
| **适合人群** | 日常高频用户 | 开发者、贡献者 |

> 两个版本 CLI 接口完全一致 —— 学会一个，两个都会用。

## 📟 运行效果

### 爬取主页

```
$ gequ crawl homepage
正在爬取主页...
成功提取数据

    主页数据统计
┏━━━━━━━━━━━━┳━━━━━━┓
┃ 类型       ┃ 数量 ┃
┡━━━━━━━━━━━━╇━━━━━━┩
│ 最新搜索   │ 20   │
│ 热门关键词 │ 10   │
│ 热门歌手   │ 10   │
└────────────┴──────┘

最新搜索:
  - 谢谢你的爱
  - traveling light
  - Bad Habits
  - 你要如何我们就如何
  - 目不转睛

已保存到数据库
```

### 爬取排行榜

```
$ gequ crawl ranking new -p 3
正在爬取 新歌榜...

已保存到数据库: 0 位歌手, 10 首歌曲, 10 条排行记录

新歌榜 - 第 3/10 页
                歌曲榜 (共 100 首)
┏━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━┓
┡━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━┩
│ 21   │ 别墅                     │ 智慧baby      │
│ 22   │ Dream                    │ 羽声Roy       │
│ 23   │ 你的爱是把温柔的刀       │ 麻山伙        │
│ 24   │ 百无聊赖                 │ TF家族-杨博文 │
│ 25   │ 跟随我                   │ 额尔古纳乐队  │
│ 26   │ 可惜春风不懂我伤悲       │ 洋澜一        │
│ 27   │ 不再回头                 │ 付豪          │
│ 28   │ 我要去找从前那个我       │ 烟嗓船长      │
│ 29   │ 想你一次落一粒沙(对唱版) │ 大潞&刘书云   │
│ 30   │ 荒漠中的草(双语对唱版)   │ 弦外之音      │
└──────┴──────────────────────────┴───────────────┘
```

### 下载歌曲

```
$ gequ download song 11980
正在下载歌曲 11980...
歌曲: 半壶纱 - 刘珂矣
下载成功
音频 (AAC): downloads\半壶纱-刘珂矣.aac
歌词: downloads\半壶纱-刘珂矣.lrc
已保存下载记录到数据库 (ID: 10)
```

### 搜索歌曲

```
$ gequ search '周杰伦'
正在搜索 '周杰伦'...
找到 10 条结果

        搜索结果 (共 10 首)
┏━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━┳━━━━━┓
┃ 序号 ┃ 歌曲       ┃ 歌手   ┃ ID  ┃
┡━━━━━━╇━━━━━━━━━━━━╇━━━━━━━━╇━━━━━┩
│ 1    │ 青花瓷     │ 周杰伦 │ 553 │
│ 2    │ 晴天       │ 周杰伦 │ 326 │
│ 3    │ 稻香       │ 周杰伦 │ 333 │
│ 4    │ 花海       │ 周杰伦 │ 564 │
│ 5    │ 七里香     │ 周杰伦 │ 329 │
│ 6    │ 反方向的钟 │ 周杰伦 │ 560 │
│ 8    │ 兰亭序     │ 周杰伦 │ 555 │
│ 9    │ 搁浅       │ 周杰伦 │ 554 │
│ 10   │ 一路向北   │ 周杰伦 │ 170 │
└──────┴────────────┴────────┴─────┘

已保存到数据库: 10 首歌曲
```

### 数据库统计

```
$ gequ db stats
      数据库统计
┏━━━━━━━━━━━━━━┳━━━━━━┓
┃ 类型         ┃ 数量 ┃
┡━━━━━━━━━━━━━━╇━━━━━━┩
│ 歌手数       │ 18   │
│ 歌曲数       │ 84   │
│ 排行记录数   │ 100  │
│ 搜索关键词数 │ 115  │
│ 下载记录数   │ 10   │
│ 页面快照数   │ 21   │
│ 页面条目数   │ 280  │
└──────────────┴──────┘
```

### 热门排行

```
$ gequ stats top-songs
                          热门歌曲 TOP 10
┏━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━┓
┃ 排名 ┃ 歌曲                             ┃ 歌手       ┃ 出现次数 ┃
┡━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━╇━━━━━━━━━━┩
│ 1    │ 不要在我寂寞的时候说爱我(Funk版) │ Tokens     │ 4        │
│ 2    │ 菩提树下许无悔                   │ 婉婷       │ 4        │
│ 3    │ Someone to Love                  │ 严浩翔     │ 3        │
│ 4    │ Run Wild（向风而野）             │ 是晚星呀   │ 3        │
│ 5    │ 今天的我们                       │ 周深       │ 3        │
│ 6    │ 一生惦念的人 (有些人不会再遇见)  │ 烟嗓船长   │ 3        │
│ 7    │ 淋雨的人                         │ 阿图表妹   │ 3        │
│ 8    │ 平凡的日子借一点光               │ 刘宇宁     │ 3        │
│ 9    │ 等晴天                           │ 周深       │ 3        │
│ 10   │ 潇洒醉风行                       │ 口水歌永兴 │ 3        │
└──────┴──────────────────────────────────┴────────────┴──────────┘
```

### 配置查看

```
$ gequ config show
          配置信息
┏━━━━━━━━━━━━━━━┳━━━━━━━━━━━┓
┃ 键            ┃ 值        ┃
┡━━━━━━━━━━━━━━━╇━━━━━━━━━━━┩
│ cookie        │ (未设置)  │
│ db_path       │ gequke.db │
│ download_dir  │ downloads │
│ output_format │ table     │
│ timeout       │ 30.0      │
└───────────────┴───────────┘

配置文件: %APPDATA%/gequ/config.json
```

## 🚀 快速开始

### Rust 版（推荐）

```bash
cd gequke-rs
cargo install --path .
gequ --help
```

> 首次安装后得到一个独立的 `gequ.exe`，无需任何运行时环境。

### Python 版

```bash
cd gequke-py
# 使用 pip
pip install -e .
# 或使用 uv（推荐）
uv sync
gequ --help
```

## 📖 命令指南

```
gequ <command> [subcommand] [options]

Commands:
  crawl       爬取主页与排行榜数据
  download    下载歌曲（MP3 + 歌词 + 封面）
  search      按关键词搜索歌曲
  db          数据库查询与管理
  stats       歌曲/歌手统计与趋势分析
  config      配置管理
```

### 爬取

```bash
# 爬取主页
gequ crawl homepage

# 爬取排行榜（7 种榜单类型）
gequ crawl ranking new -p 3                # 新歌榜第 3 页
gequ crawl ranking singer -s 1 -e 10       # 歌手榜第 1-10 页
gequ crawl ranking douyin --no-db          # 抖音榜，不存数据库
gequ crawl ranking wwdj -o ranking.json    # DJ 榜，输出 JSON
```

### 下载

```bash
gequ download song 5863335                 # 按 ID 下载单首
gequ download songs 5863335 5863376        # 批量下载
gequ download song 5863335 -o my_music     # 指定输出目录
gequ download song 5863335 --no-cover      # 不嵌入封面
```

### 搜索

```bash
gequ search "周杰伦"
gequ search "清明上" -o results.json
```

### 数据库

```bash
gequ db stats                              # 数据库概览
gequ db singer                             # 列出所有歌手
gequ db singer 周杰伦                      # 查看歌手详情
gequ db song 5863335                       # 查看歌曲详情
gequ db download                           # 查看下载记录
gequ db ranking new                        # 查看新歌榜数据
```

### 统计

```bash
gequ stats                                 # 数据库总览
gequ stats top-songs                       # 热门歌曲 Top 10
gequ stats top-singers -n 20               # 热门歌手 Top 20
gequ stats top-songs --last 7d             # 最近 7 天热门
gequ stats history homepage                # 主页快照历史
```

### 配置

```bash
gequ config show                           # 显示当前配置
gequ config set cookie "your_cookie"       # 设置 Cookie
gequ config set download_dir "downloads"
gequ config get cookie                     # 获取配置项
gequ config reset                          # 重置为默认值
```

> 完整命令说明见 [Rust 版文档](gequke-rs/README.md) 或 [Python 版设计文档](gequke-py/src/gequ/DESIGN.md)。

## 🗄️ 数据库

共 7 张 SQLite 表：

| 表名 | 说明 |
|------|------|
| `singers` | 歌手信息 (按名称去重) |
| `songs` | 歌曲元数据 (按 song_id 去重) |
| `rankings` | 7 类排行榜数据 |
| `page_snapshots` | 页面快照 — 记录每次爬取 |
| `page_items` | 快照中的歌曲条目 |
| `search_keywords` | 搜索关键词历史 |
| `downloads` | 下载记录 |

## 🛠️ 技术栈

**Rust** — Tokio, Reqwest, Scraper, Rusqlite, Clap, Serde, Tabled

**Python** — httpx, BeautifulSoup4, Typer, Rich, Mutagen

## 📁 项目结构

```
gequke-downloader/
├── gequke-rs/           # Rust 实现
│   └── src/
│       ├── cli.rs
│       ├── crawlers/    # 主页 / 排行榜 / 搜索 / 下载 爬虫
│       ├── database.rs
│       ├── config.rs
│       └── models.rs
├── gequke-py/           # Python 实现
│   └── src/gequ/
│       ├── cli.py
│       ├── crawlers/
│       ├── database.py
│       └── config.py
├── README.md            # 英文文档
├── README.zh-CN.md      # 中文文档
└── playground/          # 早期实验脚本
```

## 📄 License

MIT
