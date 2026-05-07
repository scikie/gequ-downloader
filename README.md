# gequke-downloader

歌曲客网站爬虫 - 下载歌曲和歌词

爬虫已实现。歌词下载成功。
API返回403（反爬虫），需要浏览器Cookie才能下载MP3。使用方法：
# 基本用法（下载歌词）
uv run main.py 5301
# 使用浏览器Cookie下载MP3
uv run main.py 5301 -c "Hm_tf_nmhvplng5qm=...; PHPSESSID=...; server_name_session=..."
获取Cookie步骤：
1. 浏览器打开 https://www.gequke.com/song/5301
2. F12打开开发者工具 → Application → Cookies → www.gequke.com
3. 复制所有Cookie值（如name1=value1; name2=value2格式）
