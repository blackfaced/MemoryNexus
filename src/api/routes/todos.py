# ============================================
# MemoryNexus TODO 路由
# ============================================

from fastapi import APIRouter
from pydantic import BaseModel
from typing import List, Optional
from datetime import datetime
import uuid

router = APIRouter()

class TodoCreate(BaseModel):
    title: str
    description: Optional[str] = None
    priority: str = "medium"
    due_date: Optional[datetime] = None

class TodoResponse(BaseModel):
    id: str
    title: str
    description: Optional[str]
    status: str
    priority: str
    due_date: Optional[datetime]
    created_at: datetime

todos_db = {}

@router.get("", response_model=List[TodoResponse])
async def list_todos():
    return list(todos_db.values())

@router.post("", response_model=TodoResponse)
async def create_todo(todo: TodoCreate):
    t = {
        "id": str(uuid.uuid4()),
        "title": todo.title,
        "description": todo.description,
        "status": "pending",
        "priority": todo.priority,
        "due_date": todo.due_date,
        "created_at": datetime.utcnow(),
    }
    todos_db[t["id"]] = t
    return t

@router.patch("/{todo_id}/complete")
async def complete_todo(todo_id: str):
    if todo_id in todos_db:
        todos_db[todo_id]["status"] = "completed"
    return {"message": "完成"}
