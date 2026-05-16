# ============================================
# MemoryNexus API 主入口
# ============================================

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from contextlib import asynccontextmanager

from src.core.config import settings
from src.api.routes import auth, users, memories, ai, todos, reminders, search
from src.core.database import init_db

@asynccontextmanager
async def lifespan(app: FastAPI):
    """应用生命周期管理"""
    # 启动时
    await init_db()
    yield
    # 关闭时
    pass

app = FastAPI(
    title="MemoryNexus API",
    description="🧠 家庭AI记忆中心 - 让记忆连接，让知识生长",
    version="0.1.0",
    docs_url="/docs",
    redoc_url="/redoc",
    lifespan=lifespan,
)

# 中间件
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.CORS_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# 路由
app.include_router(auth.router, prefix="/api/v1/auth", tags=["认证"])
app.include_router(users.router, prefix="/api/v1/users", tags=["用户"])
app.include_router(memories.router, prefix="/api/v1/memories", tags=["记忆"])
app.include_router(search.router, prefix="/api/v1/search", tags=["检索"])
app.include_router(ai.router, prefix="/api/v1/ai", tags=["AI"])
app.include_router(todos.router, prefix="/api/v1/todos", tags=["TODO"])
app.include_router(reminders.router, prefix="/api/v1/reminders", tags=["提醒"])

@app.get("/")
async def root():
    return {"message": "🧠 MemoryNexus API", "version": "0.1.0"}

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="0.0.0.0", port=8000, reload=True)
