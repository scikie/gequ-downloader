import json
from pathlib import Path
from typing import Optional
from dataclasses import dataclass, asdict


@dataclass
class GequConfig:
    cookie: str = ""
    db_path: str = "gequke.db"
    download_dir: str = "downloads"
    output_format: str = "table"
    user_agent: str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    timeout: int = 30
    
    @classmethod
    def get_config_dir(cls) -> Path:
        import os
        if os.name == "nt":
            app_data = os.getenv("APPDATA", "")
            return Path(app_data) / "gequ"
        else:
            return Path.home() / ".config" / "gequ"
    
    @classmethod
    def get_config_file(cls) -> Path:
        return cls.get_config_dir() / "config.json"
    
    @classmethod
    def load(cls) -> "GequConfig":
        config_file = cls.get_config_file()
        if config_file.exists():
            try:
                with open(config_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                return cls(**data)
            except Exception:
                pass
        return cls()
    
    def save(self):
        config_file = self.get_config_file()
        config_file.parent.mkdir(parents=True, exist_ok=True)
        with open(config_file, "w", encoding="utf-8") as f:
            json.dump(asdict(self), f, ensure_ascii=False, indent=2)
    
    def get(self, key: str) -> Optional[str]:
        return getattr(self, key, None)
    
    def set(self, key: str, value: str):
        if hasattr(self, key):
            setattr(self, key, value)
            self.save()
    
    def reset(self):
        default = GequConfig()
        for key in asdict(self):
            setattr(self, key, getattr(default, key))
        self.save()