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
