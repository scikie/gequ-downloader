import re
from pathlib import Path

import requests
from bs4 import BeautifulSoup


class GequkeDownloader:
    def __init__(self, output_dir: str = "downloads", cookies: str = None):
        self.session = requests.Session()
        self.session.headers.update({
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
            "Accept-Encoding": "gzip, deflate, br, zstd",
        })
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        
        if cookies:
            self._parse_cookies(cookies)
            print(f"使用浏览器Cookie: {self.session.cookies.get_dict()}")
        else:
            print("未提供Cookie，API可能返回403")
    
    def _parse_cookies(self, cookies_str: str):
        for cookie in cookies_str.split(";"):
            cookie = cookie.strip()
            if "=" in cookie:
                name, value = cookie.split("=", 1)
                self.session.cookies.set(name.strip(), value.strip(), domain="www.gequke.com")
    
    def _print_extra_url(self, extra_url: str):
        import base64
        fixed_url = extra_url.replace("#", "E")
        try:
            decoded = base64.b64decode(fixed_url).decode("utf-8")
            decoded = decoded.replace("hDtpC", "https")
            print(f"备用下载链接: {decoded}")
        except:
            print(f"备用链接 (base64编码): {fixed_url}")
    
    def get_song_page(self, song_id: int) -> BeautifulSoup:
        url = f"https://www.gequke.com/song/{song_id}"
        resp = self.session.get(url)
        resp.raise_for_status()
        resp.encoding = "utf-8"
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
    
    def get_mp3_url(self, play_id: str, song_id: int) -> str:
        api_url = "https://www.gequke.com/api/music"
        
        headers = {
            "Accept": "application/json, text/javascript, */*; q=0.01",
            "Accept-Encoding": "gzip, deflate, br, zstd",
            "Accept-Language": "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7",
            "Cache-Control": "no-cache",
            "Pragma": "no-cache",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "X-Requested-With": "XMLHttpRequest",
            "X-Custom-Header": "SecretKey",
            "Origin": "https://www.gequke.com",
            "Referer": f"https://www.gequke.com/song/{song_id}",
            "Sec-Ch-Ua": '"Microsoft Edge";v="147", "Not.A/Brand";v="8", "Chromium";v="147"',
            "Sec-Ch-Ua-Mobile": "?0",
            "Sec-Ch-Ua-Platform": '"Windows"',
            "Sec-Fetch-Dest": "empty",
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Site": "same-origin",
            "Priority": "u=1, i",
        }
        
        data = f"id={play_id}&type=0"
        
        resp = self.session.post(api_url, data=data, headers=headers)
        
        if resp.status_code != 200:
            print(f"API请求失败: {resp.status_code}")
            return None
        
        result = resp.json()
        if result.get("code") == 200:
            return result["data"]["url"]
        else:
            print(f"API返回错误: {result}")
            return None
    
    def download_file(self, url: str, filepath: Path):
        headers = {"Referer": "https://www.gequke.com/"}
        resp = self.session.get(url, stream=True, headers=headers)
        resp.raise_for_status()
        
        total_size = int(resp.headers.get("content-length", 0))
        downloaded = 0
        
        with open(filepath, "wb") as f:
            for chunk in resp.iter_content(chunk_size=8192):
                if chunk:
                    f.write(chunk)
                    downloaded += len(chunk)
                    if total_size > 0:
                        percent = (downloaded / total_size) * 100
                        print(f"\r下载进度: {percent:.1f}%", end="")
        
        print(f"\n已保存到: {filepath}")
    
    def download_song(self, song_id: int):
        print(f"正在获取歌曲页面: {song_id}")
        soup = self.get_song_page(song_id)
        
        song_info = self.extract_song_info(soup)
        if not song_info.get("play_id"):
            print("错误: 无法从页面提取歌曲信息")
            return
        
        title = song_info.get("mp3_title", "Unknown")
        author = song_info.get("mp3_author", "Unknown")
        print(f"歌曲: {title} - {author}")
        
        mp3_filename = f"{title}-{author}.mp3"
        mp3_filepath = self.output_dir / mp3_filename
        
        lrc_filename = f"{title}-{author}.lrc"
        lrc_filepath = self.output_dir / lrc_filename
        
        print(f"正在获取MP3下载链接...")
        try:
            mp3_url = self.get_mp3_url(song_info["play_id"], song_id)
            if mp3_url:
                print(f"正在下载MP3...")
                self.download_file(mp3_url, mp3_filepath)
            else:
                print("无法获取MP3下载链接")
                if song_info.get("mp3_extra_url"):
                    self._print_extra_url(song_info["mp3_extra_url"])
        except Exception as e:
            print(f"下载MP3失败: {e}")
            if song_info.get("mp3_extra_url"):
                self._print_extra_url(song_info["mp3_extra_url"])
        
        if song_info.get("lrc"):
            with open(lrc_filepath, "w", encoding="utf-8") as f:
                f.write(song_info["lrc"])
            print(f"歌词已保存到: {lrc_filepath}")


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="歌曲客网站爬虫")
    parser.add_argument("song_id", type=int, help="歌曲ID")
    parser.add_argument("-c", "--cookies", type=str, help="浏览器Cookie字符串")
    parser.add_argument("-o", "--output", type=str, default="downloads", help="输出目录")
    
    args = parser.parse_args()
    
    downloader = GequkeDownloader(output_dir=args.output, cookies=args.cookies)
    downloader.download_song(args.song_id)


if __name__ == "__main__":
    main()
