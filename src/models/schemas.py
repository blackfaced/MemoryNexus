# ============================================
# MemoryNexus 数据模型
# ============================================

from sqlalchemy import Column, String, Text, DateTime, Boolean, ForeignKey, Enum, JSON, Float
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.orm import relationship
from datetime import datetime
import uuid
import enum

from src.core.database import Base

class UserRole(str, enum.Enum):
    CHILD = "child"
    PARENT = "parent"
    FAMILY_ADMIN = "family_admin"

class MemoryType(str, enum.Enum):
    TEXT = "text"
    IMAGE = "image"
    AUDIO = "audio"
    VIDEO = "video"
    MIXED = "mixed"

class TodoStatus(str, enum.Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"

# ---- 用户相关 ----
class Family(Base):
    __tablename__ = "families"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    name = Column(String(100), nullable=False)
    settings = Column(JSON, default={})
    created_at = Column(DateTime, default=datetime.utcnow)
    
    users = relationship("User", back_populates="family")
    memories = relationship("Memory", back_populates="family")
    reminders = relationship("Reminder", back_populates="family")

class User(Base):
    __tablename__ = "users"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    email = Column(String(255), unique=True, nullable=False, index=True)
    hashed_password = Column(String(255), nullable=False)
    name = Column(String(100), nullable=False)
    role = Column(Enum(UserRole), default=UserRole.PARENT)
    family_id = Column(UUID(as_uuid=True), ForeignKey("families.id"), nullable=True)
    settings = Column(JSON, default={})
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    family = relationship("Family", back_populates="users")
    memories = relationship("Memory", back_populates="user")
    todos = relationship("Todo", back_populates="user")
    reminders = relationship("Reminder", back_populates="user")

# ---- 记忆相关 ----
class Memory(Base):
    __tablename__ = "memories"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id"), nullable=False)
    family_id = Column(UUID(as_uuid=True), ForeignKey("families.id"), nullable=True)
    type = Column(Enum(MemoryType), default=MemoryType.TEXT)
    title = Column(String(255), nullable=True)
    content = Column(Text, nullable=True)
    metadata = Column(JSON, default={})
    source = Column(String(50), default="manual")
    is_shared = Column(Boolean, default=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    user = relationship("User", back_populates="memories")
    family = relationship("Family", back_populates="memories")
    tags = relationship("Tag", secondary="memory_tags", back_populates="memories")

class Tag(Base):
    __tablename__ = "tags"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id"), nullable=True)
    family_id = Column(UUID(as_uuid=True), ForeignKey("families.id"), nullable=True)
    name = Column(String(50), nullable=False)
    category = Column(String(50), nullable=True)
    color = Column(String(10), nullable=True)
    
    memories = relationship("Memory", secondary="memory_tags", back_populates="tags")

class MemoryTag(Base):
    __tablename__ = "memory_tags"
    
    memory_id = Column(UUID(as_uuid=True), ForeignKey("memories.id"), primary_key=True)
    tag_id = Column(UUID(as_uuid=True), ForeignKey("tags.id"), primary_key=True)

# ---- TODO 相关 ----
class Todo(Base):
    __tablename__ = "todos"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id"), nullable=False)
    title = Column(String(255), nullable=False)
    description = Column(Text, nullable=True)
    status = Column(Enum(TodoStatus), default=TodoStatus.PENDING)
    priority = Column(String(10), default="medium")
    due_date = Column(DateTime, nullable=True)
    source_memory_id = Column(UUID(as_uuid=True), ForeignKey("memories.id"), nullable=True)
    created_at = Column(DateTime, default=datetime.utcnow)
    completed_at = Column(DateTime, nullable=True)
    
    user = relationship("User", back_populates="todos")

# ---- 提醒相关 ----
class Reminder(Base):
    __tablename__ = "reminders"
    
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id"), nullable=False)
    family_id = Column(UUID(as_uuid=True), ForeignKey("families.id"), nullable=True)
    type = Column(String(20), default="scheduled")  # scheduled, ai_suggested, due_date
    title = Column(String(255), nullable=False)
    content = Column(Text, nullable=True)
    trigger_at = Column(DateTime, nullable=False)
    status = Column(String(20), default="active")
    memory_id = Column(UUID(as_uuid=True), ForeignKey("memories.id"), nullable=True)
    created_at = Column(DateTime, default=datetime.utcnow)
    
    user = relationship("User", back_populates="reminders")
    family = relationship("Family", back_populates="reminders")
