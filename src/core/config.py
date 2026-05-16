# ============================================
# MemoryNexus 配置文件
# ============================================

from pydantic_settings import BaseSettings
from typing import List
import os

class Settings(BaseSettings):
    """应用配置"""
    
    # 数据库
    DATABASE_URL: str = "postgresql://postgres:postgres@localhost:5432/memorynexus"
    
    # Redis
    REDIS_URL: str = "redis://localhost:6379/0"
    
    # Qdrant 向量数据库
    QDRANT_URL: str = "http://localhost:6333"
    
    # 安全
    SECRET_KEY: str = "change-this-in-production"
    ALGORITHM: str = "HS256"
    ACCESS_TOKEN_EXPIRE_MINUTES: int = 30
    
    # AI
    OPENAI_API_KEY: str = ""
    OPENAI_MODEL: str = "gpt-4o"
    OLLAMA_BASE_URL: str = "http://localhost:11434"
    OLLAMA_MODEL: str = "qwen2.5"
    EMBEDDING_MODEL: str = "text-embedding-3-small"
    
    # CORS
    CORS_ORIGINS: List[str] = ["http://localhost:3000"]
    
    # MinIO
    MINIO_ENDPOINT: str = "localhost:9000"
    MINIO_USER: str = "minioadmin"
    MINIO_PASSWORD: str = "minioadmin"
    MINIO_BUCKET: str = "memorynexus"
    
    class Config:
        env_file = ".env"
        case_sensitive = True

settings = Settings()
