import re
from pathlib import Path

import httpx
from bs4 import BeautifulSoup


class DownloadCrawler:
    def __init__(self, output_dir: str = "downloads", cookie: str = None, user_agent: str = None, timeout: int = 30):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        self.cookie = cookie
        self.user_agent = user_agent or "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        self.timeout = timeout
    
    def _get_headers(self) -> dict:
        return {
            "User-Agent": self.user_agent,
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        }
    
    def _get_cookies(self) -> dict:
        if not self.cookie:
            return {}
        cookies = {}
        for cookie_str in self.cookie.split(";"):
            cookie_str = cookie_str.strip()
            if "=" in cookie_str:
                name, value = cookie_str.split("=", 1)
                cookies[name.strip()] = value.strip()
        return cookies
    
    async def get_song_page(self, song_id: int) -> BeautifulSoup:
        url = f"https://www.gequke.com/song/{song_id}"
        async with httpx.AsyncClient(timeout=self.timeout, follow_redirects=True) as client:
            resp = await client.get(
                url,
                headers=self._get_headers(),
                cookies=self._get_cookies()
            )
            resp.raise_for_status()
            return BeautifulSoup(resp.text, "lxml")
    
    def extract_song_info(self, soup: BeautifulSoup) -> dict:
        script_tags = soup.find_all("script")
        song_info = {}
        
        for script in script_tags:
            if script.string and "window.mp3_id" in script.string:
                script_text = script.string
                
                patterns = {
                    "mp3_id": r"window\.mp3_id\s*=\s*'([^']+)'",
                    "play_id": r"window\.play_id\s*=\s*'([^']+)'",
                    "mp3_title": r"window\.mp3_title\s*=\s*'([^']+)'",
                    "mp3_author": r"window\.mp3_author\s*=\s*'([^']+)'",
                    "mp3_cover": r"window\.mp3_cover\s*=\s*'([^']+)'",
                    "mp3_extra_url": r"window\.mp3_extra_url\s*=\s*'([^']+)'",
                }
                
                for key, pattern in patterns.items():
                    match = re.search(pattern, script_text)
                    if match:
                        song_info[key] = match.group(1)
                break
        
        content_lrc2 = soup.find("div", {"id": "content-lrc2"})
        if content_lrc2:
            lrc_html = content_lrc2.decode_contents()
            lrc_text = lrc_html.replace("<br/>", "\n").replace("<br />", "\n")
            song_info["lrc"] = lrc_text
        
        return song_info
    
    async def get_mp3_url(self, play_id: str, song_id: int) -> str:
        api_url = "https://www.gequke.com/api/music"
        
        headers = {
            **self._get_headers(),
            "Accept": "application/json, text/javascript, */*; q=0.01",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "X-Requested-With": "XMLHttpRequest",
            "X-Custom-Header": "SecretKey",
            "Origin": "https://www.gequke.com",
            "Referer": f"https://www.gequke.com/song/{song_id}",
        }
        
        data = f"id={play_id}&type=0"
        
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.post(
                api_url,
                data=data,
                headers=headers,
                cookies=self._get_cookies()
            )
            
            if resp.status_code != 200:
                return None
            
            result = resp.json()
            if result.get("code") == 200:
                return result["data"]["url"]
        
        return None
    
    async def download_file(self, url: str, filepath: Path):
        async with httpx.AsyncClient(timeout=self.timeout, follow_redirects=True) as client:
            resp = await client.get(
                url,
                headers={
                    "Referer": "https://www.gequke.com/",
                    "User-Agent": self.user_agent,
                },
                cookies=self._get_cookies()
            )
            resp.raise_for_status()
            
            with open(filepath, "wb") as f:
                f.write(resp.content)
    
    async def download_cover(self, cover_url: str) -> bytes:
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.get(cover_url)
            resp.raise_for_status()
            return resp.content
    
    def embed_mp3_metadata(self, mp3_path: Path, title: str, author: str, cover_data: bytes):
        from mutagen.mp3 import MP3
        from mutagen.id3 import ID3, TIT2, TPE1, TALB, APIC, ID3NoHeaderError
        
        try:
            audio = MP3(mp3_path, ID3=ID3)
        except ID3NoHeaderError:
            audio = MP3(mp3_path)
            audio.add_tags()
        
        audio.tags.add(TIT2(encoding=3, text=title))
        audio.tags.add(TPE1(encoding=3, text=author))
        audio.tags.add(TALB(encoding=3, text=title))
        
        if cover_data:
            audio.tags.add(APIC(
                encoding=3,
                mime='image/jpeg',
                type=3,
                desc='Cover',
                data=cover_data
            ))
        
        audio.save()
    
    async def download_song(self, song_id: int, embed_cover: bool = True) -> dict:
        result = {"success": False, "mp3_path": None, "lrc_path": None, "error": None}
        
        try:
            soup = await self.get_song_page(song_id)
        except Exception as e:
            result["error"] = f"获取歌曲页面失败: {e}"
            return result
        
        song_info = self.extract_song_info(soup)
        
        if not song_info:
            result["error"] = "无法提取歌曲信息"
            return result
        
        if not song_info.get("play_id"):
            result["error"] = "未找到 play_id，可能需要登录"
            return result
        
        title = song_info.get("mp3_title", "Unknown")
        author = song_info.get("mp3_author", "Unknown")
        
        console = None
        try:
            from rich.console import Console
            console = Console()
            console.print(f"[cyan]歌曲: {title} - {author}[/cyan]")
        except:
            pass
        
        mp3_filename = f"{title}-{author}.mp3"
        mp3_filepath = self.output_dir / mp3_filename
        
        lrc_filename = f"{title}-{author}.lrc"
        lrc_filepath = self.output_dir / lrc_filename
        
        cover_data = None
        if embed_cover and song_info.get("mp3_cover"):
            try:
                cover_data = await self.download_cover(song_info["mp3_cover"])
            except Exception as e:
                if console:
                    console.print(f"[yellow]下载封面失败: {e}[/yellow]")
        
        try:
            mp3_url = await self.get_mp3_url(song_info["play_id"], song_id)
            if not mp3_url:
                result["error"] = "API 未返回 MP3 链接"
                if song_info.get("mp3_extra_url"):
                    result["error"] += f"，备用链接: {song_info['mp3_extra_url']}"
                return result
            
            await self.download_file(mp3_url, mp3_filepath)
            
            if cover_data:
                self.embed_mp3_metadata(mp3_filepath, title, author, cover_data)
            
            result["success"] = True
            result["mp3_path"] = str(mp3_filepath)
            
        except Exception as e:
            result["error"] = f"下载 MP3 失败: {e}"
            return result
        
        if song_info.get("lrc"):
            try:
                with open(lrc_filepath, "w", encoding="utf-8") as f:
                    f.write(song_info["lrc"])
                result["lrc_path"] = str(lrc_filepath)
            except Exception as e:
                if console:
                    console.print(f"[yellow]保存歌词失败: {e}[/yellow]")
        
        return result