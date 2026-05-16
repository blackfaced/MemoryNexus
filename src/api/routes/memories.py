# ============================================
# MemoryNexus 记忆路由
# ============================================

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from typing import List, Optional
from datetime import datetime
import uuid

router = APIRouter()

# ---- Pydantic 模型 ----
class MemoryCreate(BaseModel):
    title: Optional[str] = None
    content: str
    type: str = "text"
    tags: List[str] = []
    is_shared: bool = False

class MemoryResponse(BaseModel):
    id: str
    title: Optional[str]
    content: str
    type: str
    created_at: datetime
    tags: List[str] = []

class MemoryUpdate(BaseModel):
    title: Optional[str] = None
    content: Optional[str] = None
    tags: Optional[List[str]] = None

# ---- 模拟数据 ----
memories_db = {}

@router.get("", response_model=List[MemoryResponse])
async def list_memories(skip: int = 0, limit: int = 10):
    """获取记忆列表"""
    memories = list(memories_db.values())[skip:skip+limit]
    return memories

@router.post("", response_model=MemoryResponse)
async def create_memory(memory: MemoryCreate):
    """创建记忆"""
    mem = {
        "id": str(uuid.uuid4()),
        "title": memory.title,
        "content": memory.content,
        "type": memory.type,
        "created_at": datetime.utcnow(),
        "tags": memory.tags,
    }
    memories_db[mem["id"]] = mem
    return mem

@router.get("/{memory_id}", response_model=MemoryResponse)
async def get_memory(memory_id: str):
    """获取单个记忆"""
    if memory_id not in memories_db:
        raise HTTPException(status_code=404, detail="记忆不存在")
    return memories_db[memory_id]

@router.patch("/{memory_id}", response_model=MemoryResponse)
async def update_memory(memory_id: str, memory: MemoryUpdate):
    """更新记忆"""
    if memory_id not in memories_db:
        raise HTTPException(status_code=404, detail="记忆不存在")
    
    mem = memories_db[memory_id]
    if memory.title is not None:
        mem["title"] = memory.title
    if memory.content is not None:
        mem["content"] = memory.content
    if memory.tags is not None:
        mem["tags"] = memory.tags
    
    return mem

@router.delete("/{memory_id}")
async def delete_memory(memory_id: str):
    """删除记忆"""
    if memory_id not in memories_db:
        raise HTTPException(status_code=404, detail="记忆不存在")
    del memories_db[memory_id]
    return {"message": "删除成功"}
