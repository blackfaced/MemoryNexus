# ============================================
# MemoryNexus 提醒路由
# ============================================

from fastapi import APIRouter
from pydantic import BaseModel
from typing import List, Optional
from datetime import datetime
import uuid

router = APIRouter()

class ReminderCreate(BaseModel):
    title: str
    content: Optional[str] = None
    trigger_at: datetime
    type: str = "scheduled"

class ReminderResponse(BaseModel):
    id: str
    title: str
    content: Optional[str]
    trigger_at: datetime
    status: str
    created_at: datetime

reminders_db = {}

@router.get("", response_model=List[ReminderResponse])
async def list_reminders():
    return list(reminders_db.values())

@router.post("", response_model=ReminderResponse)
async def create_reminder(reminder: ReminderCreate):
    r = {
        "id": str(uuid.uuid4()),
        "title": reminder.title,
        "content": reminder.content,
        "trigger_at": reminder.trigger_at,
        "status": "active",
        "created_at": datetime.utcnow(),
    }
    reminders_db[r["id"]] = r
    return r
