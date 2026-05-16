# ============================================
# MemoryNexus 用户路由
# ============================================

from fastapi import APIRouter, Depends
from pydantic import BaseModel

router = APIRouter()

class UserResponse(BaseModel):
    id: str
    email: str
    name: str
    role: str

@router.get("/me", response_model=UserResponse)
async def get_current_user():
    """获取当前用户"""
    return {"id": "1", "email": "user@example.com", "name": "User", "role": "parent"}
