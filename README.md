<p align="center">
  <h1 align="center">🎵 Gequ Downloader</h1>
  <p align="center">CLI tool for crawling, searching, and downloading music from gequke.com</p>
  <p align="center">
    <a href="#-features">Features</a> •
    <a href="#-dual-implementation">Dual Implementation</a> •
    <a href="#-quick-start">Quick Start</a> •
    <a href="#-commands">Commands</a> •
    <a href="#-database">Database</a>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust"/>
    <img src="https://img.shields.io/badge/Python-3.10%2B-blue?logo=python"/>
    <img src="https://img.shields.io/badge/license-MIT-green"/>
  </p>
  <p align="center">
    <a href="README.zh-CN.md">🇨🇳 中文版</a>
  </p>
</p>

---

## ✨ Features

- **Crawl** homepage and 7 ranking charts (singer, surge, new, douyin, classic, electronic, DJ)
- **Search** songs by keyword
- **Download** MP3 audio + synchronized lyrics + album cover
- **Store** everything in SQLite with 7 well-designed tables
- **Analyze** song/singer statistics and popularity trends
- **Snapshot** page history for data traceability
- **Configurable** via JSON config file

## 🦀 🐍 Dual Implementation

Available in **two flavors** — pick the one that suits you:

| Aspect | Rust (`gequke-rs/`) | Python (`gequke-py/`) |
|--------|---------------------|-----------------------|
| **Runtime** | Compiled native binary | Requires Python 3.10+ |
| **Performance** | Blazing fast | Fast enough |
| **Deployment** | Single binary (~10 MB) | Requires pip/uv |
| **Concurrency** | Tokio async | httpx async |
| **Best for** | Daily power users | Hackers & contributors |

Both share the **same CLI interface** — learn once, use both.

## 📟 Demo

### Crawl homepage

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

### Crawl ranking (new song chart, page 3)

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

### Download song

```
$ gequ download song 11980
正在下载歌曲 11980...
歌曲: 半壶纱 - 刘珂矣
下载成功
音频 (AAC): downloads\半壶纱-刘珂矣.aac
歌词: downloads\半壶纱-刘珂矣.lrc
已保存下载记录到数据库 (ID: 10)
```

### Search

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

### Database stats

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

### Top songs

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

### Config

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

## 🚀 Quick Start

### Rust

```bash
cd gequke-rs
cargo install --path .
gequ --help
```

### Python

```bash
cd gequke-py
# with pip
pip install -e .
# or with uv
uv sync
gequ --help
```

## 📖 Commands

```
gequ <command> [subcommand] [options]

Commands:
  crawl       Scrape homepage & ranking data
  download    Download songs (MP3 + lyrics + cover)
  search      Search songs by keyword
  db          SQLite database operations
  stats       Song/singer statistics & trends
  config      Manage configuration
```

### Crawl

```bash
# homepage
gequ crawl homepage

# rankings (7 types: singer, surge, new, douyin, jingdian, dianyin, wwdj)
gequ crawl ranking new -p 3
gequ crawl ranking singer -s 1 -e 10
```

### Download

```bash
gequ download song 5863335            # single song by ID
gequ download songs 5863335 5863376   # multiple songs at once
```

### Search

```bash
gequ search "周杰伦"
gequ search "清明上" -o results.json
```

### Stats

```bash
gequ stats top-songs                  # top 10 hottest songs
gequ stats top-singers                # top singers by appearances
gequ stats top-songs --last 7d        # last 7 days only
```

> See [gequke-rs/README.md](gequke-rs/README.md) or [DESIGN.md](gequke-py/src/gequ/DESIGN.md) for the full command reference.

## 🗄️ Database

| Table | Description |
|-------|-------------|
| `singers` | Artist info (unique by name) |
| `songs` | Song metadata (unique by song_id) |
| `rankings` | Chart data across 7 categories |
| `page_snapshots` | Crawl snapshots for history tracking |
| `page_items` | Individual items in each snapshot |
| `search_keywords` | Search keywords history |
| `downloads` | Download records |

## 🛠️ Tech Stack

**Rust** — Tokio, Reqwest, Scraper, Rusqlite, Clap, Serde, Tabled

**Python** — httpx, BeautifulSoup4, Typer, Rich, Mutagen

## 📁 Project Structure

```
gequke-downloader/
├── gequke-rs/           # Rust implementation
│   └── src/
│       ├── cli.rs
│       ├── crawlers/
│       ├── database.rs
│       ├── config.rs
│       └── models.rs
├── gequke-py/           # Python implementation
│   └── src/gequ/
│       ├── cli.py
│       ├── crawlers/
│       ├── database.py
│       └── config.py
└── README.md
```

## 📄 License

MIT
