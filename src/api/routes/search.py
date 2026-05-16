# ============================================
# MemoryNexus 搜索路由
# ============================================

from fastapi import APIRouter
from pydantic import BaseModel
from typing import List, Optional

router = APIRouter()

class SearchRequest(BaseModel):
    query: str
    limit: int = 10
    user_id: Optional[str] = None
    type: Optional[str] = None

class SearchResponse(BaseModel):
    results: List[dict]
    total: int

@router.post("", response_model=SearchResponse)
async def search(request: SearchRequest):
    """语义搜索"""
    # TODO: 实现向量检索
    return {"results": [], "total": 0}
