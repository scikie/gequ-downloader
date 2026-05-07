import asyncio
import json
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, TextColumn

from .config import GequConfig
from .crawlers import HomepageCrawler, RankingCrawler, SearchCrawler, DownloadCrawler

app = typer.Typer(
    name="gequ",
    help="歌曲客网站爬虫工具",
    add_completion=False
)
console = Console()
config = GequConfig.load()


crawl_app = typer.Typer(help="爬取数据")
app.add_typer(crawl_app, name="crawl")


@crawl_app.command("homepage")
def crawl_homepage(
    file: Optional[str] = typer.Option(None, "-f", "--file", help="从本地HTML文件读取"),
    output: Optional[str] = typer.Option(None, "-o", "--output", help="输出JSON文件路径"),
    no_db: bool = typer.Option(False, "--no-db", help="不保存到数据库"),
):
    """爬取主页数据"""
    crawler = HomepageCrawler(
        cookie=config.cookie,
        user_agent=config.user_agent,
        timeout=float(config.timeout)
    )
    
    console.print("正在爬取主页...")
    
    try:
        if file:
            soup = crawler.get_homepage_from_file(file)
            url = None
        else:
            soup = asyncio.run(crawler.get_homepage())
            url = "https://www.gequke.com/"
        
        data = crawler.extract_all(soup)
        console.print("[green]成功提取数据[/green]")
        
    except Exception as e:
        console.print(f"[red]爬取失败: {e}[/red]")
        raise typer.Exit(1)
    
    table = Table(title="主页数据统计")
    table.add_column("类型", style="cyan")
    table.add_column("数量", style="magenta")
    table.add_row("最新搜索", str(len(data.latest_searches)))
    table.add_row("热门关键词", str(len(data.hot_keywords)))
    table.add_row("热门歌手", str(len(data.hot_singers)))
    console.print(table)
    
    if data.latest_searches:
        console.print("\n[bold]最新搜索:[/bold]")
        for item in data.latest_searches[:5]:
            console.print(f"  - {item.keyword}")
    
    if not no_db:
        from .database import Database
        from .models import SearchKeyword, Singer, PageSnapshot, PageItem
        
        db = Database(config.db_path)
        
        snapshot = PageSnapshot(
            page_type="homepage",
            url=url,
            title="歌曲客主页"
        )
        snapshot_id = db.insert_page_snapshot(snapshot)
        
        keywords = [
            SearchKeyword(keyword=item.keyword, source="latest")
            for item in data.latest_searches
        ]
        db.insert_search_keywords(keywords)
        
        page_items = []
        for item in data.hot_keywords:
            page_items.append(PageItem(
                page_snapshot_id=snapshot_id,
                item_type="keyword",
                position=item.rank,
                extra_data=json.dumps({"keyword": item.keyword, "url": item.url})
            ))
        
        for item in data.hot_singers:
            singer = Singer(name=item.name, songs_url=item.url)
            singer_id = db.insert_singer(singer)
            page_items.append(PageItem(
                page_snapshot_id=snapshot_id,
                item_type="singer",
                item_id=singer_id,
                position=item.rank,
                extra_data=json.dumps({"url": item.url})
            ))
        
        db.insert_page_items(page_items)
        console.print("\n[green]已保存到数据库[/green]")
    
    if output:
        crawler.save_to_json(data, output)
        console.print(f"\n[green]已保存到: {output}[/green]")


@crawl_app.command("ranking")
def crawl_ranking(
    ranking_type: str = typer.Argument(..., help=f"榜单类型: {', '.join(RankingCrawler.RANKING_TYPES.keys())}"),
    page: int = typer.Option(1, "-p", "--page", help="页码"),
    start_page: Optional[int] = typer.Option(None, "-s", "--start-page", help="起始页码（多页爬取）"),
    end_page: Optional[int] = typer.Option(None, "-e", "--end-page", help="结束页码（多页爬取）"),
    file: Optional[str] = typer.Option(None, "-f", "--file", help="从本地HTML文件读取"),
    output: Optional[str] = typer.Option(None, "-o", "--output", help="输出JSON文件路径"),
    no_db: bool = typer.Option(False, "--no-db", help="不保存到数据库"),
):
    """爬取排行榜数据"""
    if ranking_type not in RankingCrawler.RANKING_TYPES:
        console.print(f"[red]错误: 无效的榜单类型 '{ranking_type}'[/red]")
        console.print(f"支持的榜单: {', '.join(RankingCrawler.RANKING_TYPES.keys())}")
        raise typer.Exit(1)
    
    crawler = RankingCrawler(
        cookie=config.cookie,
        user_agent=config.user_agent,
        timeout=float(config.timeout)
    )
    
    ranking_name = RankingCrawler.RANKING_TYPES[ranking_type]
    
    try:
        if file:
            console.print("从本地文件读取...")
            soup = crawler.get_ranking_page_from_file(file)
            data = crawler.extract_all(soup, page)
            all_data = [data]
            pages_processed = [page]
        elif start_page and end_page:
            all_data = []
            pages_processed = []
            for p in range(start_page, end_page + 1):
                console.print(f"正在爬取 {ranking_name} 第 {p} 页...")
                soup = asyncio.run(crawler.get_ranking_page(ranking_type, p))
                data = crawler.extract_all(soup, p)
                all_data.append(data)
                pages_processed.append(p)
            
            console.print(f"[green]成功爬取 {end_page - start_page + 1} 页[/green]")
            
            if output:
                if all_data[0].singers:
                    all_items = [item for d in all_data for item in d.singers]
                else:
                    all_items = [item for d in all_data for item in d.songs]
                
                output_path = Path(output)
                combined_data = {
                    "ranking_name": ranking_name,
                    "start_page": start_page,
                    "end_page": end_page,
                    "total_items": len(all_items),
                    "items": [item.__dict__ for item in all_items]
                }
                
                output_path.parent.mkdir(parents=True, exist_ok=True)
                with open(output_path, "w", encoding="utf-8") as f:
                    json.dump(combined_data, f, ensure_ascii=False, indent=2)
                
                console.print(f"[green]已保存到: {output_path}[/green]")
        else:
            console.print(f"正在爬取 {ranking_name}...")
            soup = asyncio.run(crawler.get_ranking_page(ranking_type, page))
            data = crawler.extract_all(soup, page)
            all_data = [data]
            pages_processed = [page]
            console.print("[green]成功提取数据[/green]")
            
    except Exception as e:
        console.print(f"[red]爬取失败: {e}[/red]")
        raise typer.Exit(1)
    
    if not no_db:
        from .database import Database
        from .models import Song, Singer, RankingItem, PageSnapshot, PageItem
        
        db = Database(config.db_path)
        
        song_count = 0
        singer_count = 0
        ranking_count = 0
        
        for idx, data in enumerate(all_data):
            page_number = pages_processed[idx]
            
            snapshot = PageSnapshot(
                page_type="ranking",
                ranking_type=ranking_type,
                page_number=page_number,
                title=data.ranking_name
            )
            snapshot_id = db.insert_page_snapshot(snapshot)
            
            page_items = []
            
            if data.singers:
                for singer in data.singers:
                    s = Singer(
                        name=singer.name,
                        avatar_url=singer.avatar_url,
                        songs_url=singer.songs_url
                    )
                    singer_id = db.insert_singer(s)
                    singer_count += 1
                    
                    r = RankingItem(
                        ranking_type=ranking_type,
                        rank=singer.rank,
                        item_name=singer.name,
                        item_type="singer"
                    )
                    db.insert_ranking_item(r)
                    ranking_count += 1
                    
                    page_items.append(PageItem(
                        page_snapshot_id=snapshot_id,
                        item_type="singer",
                        item_id=singer_id,
                        position=singer.rank,
                        extra_data=json.dumps({
                            "avatar_url": singer.avatar_url,
                            "songs_url": singer.songs_url
                        })
                    ))
            else:
                for song in data.songs:
                    s = Song(
                        song_id=song.song_id,
                        title=song.title,
                        artist=song.artist,
                        cover_url=song.cover_url
                    )
                    song_db_id = db.insert_song(s)
                    song_count += 1
                    
                    r = RankingItem(
                        ranking_type=ranking_type,
                        rank=song.rank,
                        item_id=song.song_id,
                        item_type="song"
                    )
                    db.insert_ranking_item(r)
                    ranking_count += 1
                    
                    page_items.append(PageItem(
                        page_snapshot_id=snapshot_id,
                        item_type="song",
                        item_id=song_db_id,
                        position=song.rank,
                        extra_data=json.dumps({
                            "title": song.title,
                            "artist": song.artist,
                            "cover_url": song.cover_url
                        })
                    ))
            
            db.insert_page_items(page_items)
        
        console.print(f"\n[green]已保存到数据库: {singer_count} 位歌手, {song_count} 首歌曲, {ranking_count} 条排行记录[/green]")
    
    if output:
        if len(all_data) == 1:
            crawler.save_to_json(all_data[0], output)
            console.print(f"\n[green]已保存到: {output}[/green]")
    
    if len(all_data) == 1:
        data = all_data[0]
        console.print(f"\n[bold]{data.ranking_name}[/bold] - 第 {data.pagination.current_page}/{data.pagination.total_pages} 页")
        
        if data.singers:
            table = Table(title=f"歌手榜 (共 {data.pagination.total_songs} 位)")
            table.add_column("排名", style="cyan")
            table.add_column("歌手", style="magenta")
            for singer in data.singers[:10]:
                table.add_row(str(singer.rank), singer.name)
        else:
            table = Table(title=f"歌曲榜 (共 {data.pagination.total_songs} 首)")
            table.add_column("排名", style="cyan")
            table.add_column("歌曲", style="green")
            table.add_column("歌手", style="magenta")
            for song in data.songs[:10]:
                table.add_row(str(song.rank), song.title, song.artist)
        
        console.print(table)


download_app = typer.Typer(help="下载歌曲")
app.add_typer(download_app, name="download")


@download_app.command("song")
def download_song(
    song_id: int = typer.Argument(..., help="歌曲ID"),
    output_dir: Optional[str] = typer.Option(None, "-o", "--output", help="输出目录"),
    no_cover: bool = typer.Option(False, "--no-cover", help="不嵌入封面"),
    no_db: bool = typer.Option(False, "--no-db", help="不保存下载记录到数据库"),
):
    """下载单首歌曲"""
    from pathlib import Path
    
    output = output_dir or config.download_dir
    crawler = DownloadCrawler(
        output_dir=output,
        cookie=config.cookie,
        user_agent=config.user_agent,
        timeout=float(config.timeout)
    )
    
    console.print(f"正在下载歌曲 {song_id}...")
    
    try:
        result = asyncio.run(crawler.download_song(song_id, embed_cover=not no_cover))
        
        if result["success"]:
            console.print("[green]下载成功[/green]")
            if result["mp3_path"]:
                audio_path = Path(result["mp3_path"])
                ext = audio_path.suffix.lstrip('.').upper()
                console.print(f"[green]音频 ({ext}): {result['mp3_path']}[/green]")
            if result["lrc_path"]:
                console.print(f"[green]歌词: {result['lrc_path']}[/green]")
            
            if not no_db and result["mp3_path"]:
                from .database import Database
                from .models import DownloadRecord, Song
                
                db = Database(config.db_path)
                
                # 先保存歌曲信息到数据库（如果不存在）
                if result.get("song_info"):
                    song_data = result["song_info"]
                    song = Song(
                        song_id=song_data["song_id"],
                        title=song_data["title"],
                        artist=song_data["artist"],
                        cover_url=song_data.get("cover_url"),
                        play_id=song_data.get("play_id"),
                    )
                    db.insert_song(song)
                
                mp3_path = Path(result["mp3_path"])
                file_size = mp3_path.stat().st_size if mp3_path.exists() else None
                
                record = DownloadRecord(
                    song_id=song_id,
                    file_path=result["mp3_path"],
                    file_size=file_size
                )
                record_id = db.insert_download_record(record)
                
                if record_id > 0:
                    console.print(f"[green]已保存下载记录到数据库 (ID: {record_id})[/green]")
        else:
            console.print(f"[red]下载失败[/red]")
            if result.get("error"):
                console.print(f"[red]原因: {result['error']}[/red]")
            raise typer.Exit(1)
            
    except Exception as e:
        console.print(f"[red]下载失败: {e}[/red]")
        import traceback
        console.print(f"[red]{traceback.format_exc()}[/red]")
        raise typer.Exit(1)


@download_app.command("songs")
def download_songs(
    song_ids: list[int] = typer.Argument(..., help="歌曲ID列表"),
    output_dir: Optional[str] = typer.Option(None, "-o", "--output", help="输出目录"),
    no_db: bool = typer.Option(False, "--no-db", help="不保存下载记录到数据库"),
):
    """批量下载多首歌曲"""
    output = output_dir or config.download_dir
    crawler = DownloadCrawler(
        output_dir=output,
        cookie=config.cookie,
        user_agent=config.user_agent,
        timeout=float(config.timeout)
    )
    
    success_count = 0
    fail_count = 0
    
    for song_id in song_ids:
        try:
            result = asyncio.run(crawler.download_song(song_id))
            if result["success"]:
                success_count += 1
                console.print(f"[green]✓[/green] {song_id}")
                
                if not no_db and result["mp3_path"]:
                    from .database import Database
                    from .models import DownloadRecord, Song
                    from pathlib import Path
                    
                    db = Database(config.db_path)
                    
                    # 先保存歌曲信息到数据库（如果不存在）
                    if result.get("song_info"):
                        song_data = result["song_info"]
                        song = Song(
                            song_id=song_data["song_id"],
                            title=song_data["title"],
                            artist=song_data["artist"],
                            cover_url=song_data.get("cover_url"),
                            play_id=song_data.get("play_id"),
                        )
                        db.insert_song(song)
                    
                    mp3_path = Path(result["mp3_path"])
                    file_size = mp3_path.stat().st_size if mp3_path.exists() else None
                    
                    record = DownloadRecord(
                        song_id=song_id,
                        file_path=result["mp3_path"],
                        file_size=file_size
                    )
                    db.insert_download_record(record)
            else:
                fail_count += 1
                console.print(f"[red]✗[/red] {song_id}")
                if result.get("error"):
                    console.print(f"  [dim]{result['error']}[/dim]")
        except Exception as e:
            fail_count += 1
            console.print(f"[red]✗[/red] {song_id}: {e}")
    
    console.print(f"\n[bold]完成: {success_count} 成功, {fail_count} 失败[/bold]")


db_app = typer.Typer(help="数据库操作")
app.add_typer(db_app, name="db")


@db_app.command("stats")
def db_stats():
    """显示数据库统计信息"""
    from .database import Database
    
    db = Database(config.db_path)
    stats = db.get_stats()
    
    table = Table(title="数据库统计")
    table.add_column("类型", style="cyan")
    table.add_column("数量", style="magenta")
    
    table.add_row("歌手数", str(stats['total_singers']))
    table.add_row("歌曲数", str(stats['total_songs']))
    table.add_row("排行记录数", str(stats['total_rankings']))
    table.add_row("搜索关键词数", str(stats['total_keywords']))
    table.add_row("下载记录数", str(stats['total_downloads']))
    table.add_row("页面快照数", str(stats['total_page_snapshots']))
    table.add_row("页面条目数", str(stats['total_page_items']))
    
    console.print(table)


@db_app.command("singer")
def db_singer(
    name: Optional[str] = typer.Argument(None, help="歌手名称（可选，不指定则列出所有）"),
    limit: int = typer.Option(20, "-n", "--number", help="显示数量"),
):
    """查询歌手信息"""
    from .database import Database
    
    db = Database(config.db_path)
    
    if name:
        singer = db.get_singer_by_name(name)
        if singer:
            table = Table(title=f"歌手: {name}")
            table.add_column("字段", style="cyan")
            table.add_column("值", style="magenta")
            table.add_row("ID", str(singer['id']))
            table.add_row("名称", singer['name'])
            table.add_row("头像", singer.get('avatar_url', '-') or '-')
            table.add_row("歌曲页", singer.get('songs_url', '-') or '-')
            table.add_row("创建时间", singer.get('created_at', '-'))
            console.print(table)
            
            stats = db.get_singer_appearance_stats(name)
            console.print(f"\n[bold]出现统计:[/bold]")
            console.print(f"  总次数: {stats['total_count']}")
            console.print(f"  主页: {stats['homepage_count']}")
            console.print(f"  排行榜: {stats['ranking_count']}")
        else:
            console.print(f"[red]未找到歌手: {name}[/red]")
    else:
        singers = db.get_all_singers(limit)
        if singers:
            table = Table(title=f"歌手列表 (共 {len(singers)} 位)")
            table.add_column("ID", style="cyan")
            table.add_column("歌手", style="magenta")
            table.add_column("创建时间", style="green")
            for singer in singers:
                table.add_row(str(singer['id']), singer['name'], singer.get('created_at', '-'))
            console.print(table)
        else:
            console.print("[yellow]暂无歌手数据[/yellow]")


@db_app.command("song")
def db_song(
    song_id: Optional[int] = typer.Argument(None, help="歌曲ID（可选，不指定则列出所有）"),
    limit: int = typer.Option(20, "-n", "--number", help="显示数量"),
):
    """查询歌曲信息"""
    from .database import Database
    
    db = Database(config.db_path)
    
    if song_id:
        song = db.get_song_by_id(song_id)
        if song:
            table = Table(title=f"歌曲: {song_id}")
            table.add_column("字段", style="cyan")
            table.add_column("值", style="magenta")
            table.add_row("ID", str(song['id']))
            table.add_row("歌曲ID", str(song['song_id']))
            table.add_row("标题", song['title'])
            table.add_row("歌手", song['artist'])
            table.add_row("封面", song.get('cover_url', '-') or '-')
            table.add_row("创建时间", song.get('created_at', '-'))
            console.print(table)
            
            stats = db.get_song_appearance_stats(song_id)
            console.print(f"\n[bold]出现统计:[/bold]")
            console.print(f"  总次数: {stats['total_count']}")
            console.print(f"  主页: {stats['homepage_count']}")
            console.print(f"  排行榜: {stats['ranking_count']}")
            
            if stats['appearances']:
                console.print(f"\n[bold]最近出现:[/bold]")
                for app in stats['appearances'][:5]:
                    console.print(f"  {app['page_type']} | 排名 {app['position']} | {app['crawled_at']}")
        else:
            console.print(f"[red]未找到歌曲: {song_id}[/red]")
    else:
        songs = db.get_all_songs(limit)
        if songs:
            table = Table(title=f"歌曲列表 (共 {len(songs)} 首)")
            table.add_column("歌曲ID", style="cyan")
            table.add_column("标题", style="green")
            table.add_column("歌手", style="magenta")
            table.add_column("创建时间", style="yellow")
            for song in songs:
                table.add_row(str(song['song_id']), song['title'], song['artist'], song.get('created_at', '-'))
            console.print(table)
        else:
            console.print("[yellow]暂无歌曲数据[/yellow]")


@db_app.command("download")
def db_download(
    limit: int = typer.Option(20, "-n", "--number", help="显示数量"),
):
    """查询下载历史"""
    from .database import Database
    
    db = Database(config.db_path)
    downloads = db.get_all_downloads(limit)
    
    if downloads:
        table = Table(title=f"下载历史 (共 {len(downloads)} 条)")
        table.add_column("ID", style="cyan")
        table.add_column("歌曲ID", style="yellow")
        table.add_column("标题", style="green")
        table.add_column("歌手", style="magenta")
        table.add_column("文件大小", style="blue")
        table.add_column("下载时间", style="dim")
        
        for d in downloads:
            size_str = f"{d['file_size'] // 1024}KB" if d.get('file_size') else '-'
            title = d.get('title', '-') or '-'
            artist = d.get('artist', '-') or '-'
            table.add_row(
                str(d['id']),
                str(d['song_id']),
                title,
                artist,
                size_str,
                d.get('downloaded_at', '-')
            )
        console.print(table)
    else:
        console.print("[yellow]暂无下载记录[/yellow]")


@db_app.command("ranking")
def db_ranking(
    ranking_type: Optional[str] = typer.Argument(None, help="榜单类型（可选）"),
    limit: int = typer.Option(20, "-n", "--number", help="显示数量"),
):
    """查询排行榜数据"""
    from .database import Database
    from .models import RANKING_TYPES
    
    db = Database(config.db_path)
    
    if ranking_type and ranking_type not in RANKING_TYPES:
        console.print(f"[red]无效的榜单类型: {ranking_type}[/red]")
        console.print(f"支持类型: {', '.join(RANKING_TYPES.keys())}")
        raise typer.Exit(1)
    
    rankings = db.get_all_rankings(ranking_type, limit)
    
    if rankings:
        title = f"{RANKING_TYPES.get(ranking_type, '排行榜')} (共 {len(rankings)} 条)" if ranking_type else f"排行榜数据 (共 {len(rankings)} 条)"
        table = Table(title=title)
        table.add_column("排名", style="cyan")
        table.add_column("类型", style="yellow")
        table.add_column("标题/歌手", style="green")
        table.add_column("歌手", style="magenta")
        table.add_column("抓取时间", style="dim")
        
        for r in rankings:
            ranking_name = RANKING_TYPES.get(r['ranking_type'], r['ranking_type'])
            if r.get('singer_name'):
                item_name = r['singer_name']
                artist = '-'
            else:
                item_name = r.get('title', '-') or '-'
                artist = r.get('artist', '-') or '-'
            
            table.add_row(
                str(r['rank']),
                ranking_name,
                item_name,
                artist,
                r.get('crawled_at', '-')
            )
        console.print(table)
    else:
        console.print("[yellow]暂无排行榜数据[/yellow]")


config_app = typer.Typer(help="配置管理")
app.add_typer(config_app, name="config")


@config_app.command("show")
def config_show():
    """显示当前配置"""
    table = Table(title="配置信息")
    table.add_column("键", style="cyan")
    table.add_column("值", style="magenta")
    
    table.add_row("cookie", config.cookie[:20] + "..." if config.cookie else "(未设置)")
    table.add_row("db_path", config.db_path)
    table.add_row("download_dir", config.download_dir)
    table.add_row("output_format", config.output_format)
    table.add_row("timeout", str(config.timeout))
    
    console.print(table)
    console.print(f"\n配置文件: {GequConfig.get_config_file()}")


@config_app.command("set")
def config_set(
    key: str = typer.Argument(..., help="配置键"),
    value: str = typer.Argument(..., help="配置值"),
):
    """设置配置项"""
    config.set(key, value)
    console.print(f"[green]已设置 {key} = {value}[/green]")


@config_app.command("get")
def config_get(
    key: str = typer.Argument(..., help="配置键"),
):
    """获取配置项"""
    value = config.get(key)
    if value:
        console.print(value)
    else:
        console.print("[red]配置项不存在[/red]")


@config_app.command("reset")
def config_reset():
    """重置所有配置"""
    config.reset()
    console.print("[green]配置已重置[/green]")


@app.command()
def search(
    keyword: str = typer.Argument(..., help="搜索关键词"),
    output: Optional[str] = typer.Option(None, "-o", "--output", help="输出JSON文件路径"),
):
    """搜索歌曲"""
    crawler = SearchCrawler(
        cookie=config.cookie,
        user_agent=config.user_agent,
        timeout=float(config.timeout)
    )
    
    console.print(f"正在搜索 '{keyword}'...")
    
    try:
        soup = asyncio.run(crawler.search(keyword))
        data = crawler.extract_all(soup)
        console.print(f"[green]找到 {data.total_count} 条结果[/green]")
    except Exception as e:
        console.print(f"[red]搜索失败: {e}[/red]")
        raise typer.Exit(1)
    
    if data.songs:
        table = Table(title=f"搜索结果 (共 {len(data.songs)} 首)")
        table.add_column("序号", style="cyan")
        table.add_column("歌曲", style="green")
        table.add_column("歌手", style="magenta")
        table.add_column("ID", style="yellow")
        
        for song in data.songs[:20]:
            table.add_row(str(song.position), song.title, song.artist, str(song.song_id))
        
        console.print(table)
    
    if output:
        crawler.save_to_json(data, output)
        console.print(f"\n[green]已保存到: {output}[/green]")


@app.command()
def version():
    """显示版本信息"""
    from . import __version__
    console.print(f"gequ version {__version__}")


if __name__ == "__main__":
    app()