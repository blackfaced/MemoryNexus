# 🧠 MemoryNexus - 家庭AI记忆中心

> 连接每一份记忆，编织家庭知识网络

**MemoryNexus** 是一个开源的家庭 AI 记忆系统，基于"第二大脑"概念构建。

| 🦀 **技术栈** | Rust + Axum | ⚛️ React + TS | ☁️ 云原生 |
|:---:|:---:|:---:|:---:|
| 后端 | Rust 0.72 | 前端 | React 18 |
| 存储 | S3/MinIO | 向量 | Qdrant |
| AI | Whisper + LLM | 数据库 | PostgreSQL |

📋 **架构决策**: [decisions/](decisions/) 目录下管理所有 ADR

[![GitHub stars](https://img.shields.io/github/stars/blackfaced/MemoryNexus)](https://github.com/blackfaced/MemoryNexus/stargazers)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![GitHub Actions](https://github.com/blackfaced/MemoryNexus/actions/workflows/ci.yml/badge.svg)](https://github.com/blackfaced/MemoryNexus/actions)

---

## ✨ 核心特性

| 特性 | 说明 |
|------|------|
| 🤖 **AI 主动助理** | 定时回顾、洞察发现、主动提醒、TODO 生成 |
| 👨‍👩‍👧 **多用户/家庭** | 孩子学习模式、父母工作模式、家庭共享 |
| 📦 **多模态存储** | 文字、图片、音频、视频，统一向量检索 |
| 🔒 **隐私优先** | 数据完全自主，支持本地部署 |
| ☁️ **多云部署** | Docker 一键部署，支持所有主流云 |
| 🔌 **开放生态** | 标准 API，任何 Agent 都能接入 |

---

## 🚀 快速开始

### 方式一：Docker 一键部署（推荐）

```bash
# 1. 克隆项目
git clone https://github.com/blackfaced/MemoryNexus.git
cd MemoryNexus

# 2. 一键启动
docker-compose up -d

# 3. 访问
# 前端: http://localhost:3000
# API:  http://localhost:8000
```

### 方式二：本地开发（Rust 主线）

```bash
# 1. 克隆项目
git clone https://github.com/blackfaced/MemoryNexus.git
cd MemoryNexus

# 2. 启动基础设施
docker-compose up -d postgres qdrant redis

# 3. 配置环境变量
cp .env.example .env
# 至少配置 DATABASE_URL；语义搜索还需要 OPENAI_API_KEY 和 QDRANT_URL

# 4. 启动 Rust API
cd src
cargo run
```

---

## 🎯 使用场景

```
┌─────────────────────────────────────────────────────────┐
│                      MemoryNexus                         │
├─────────────────────────────────────────────────────────┤
│  👦 孩子模式          👨‍💼 父母模式          👨‍👩‍👧 家庭模式     │
│  ─────────          ─────────          ─────────        │
│  📚 学习记录         💼 工作想法         🏠 家庭日记     │
│  📝 作业积累         📋 项目笔记         📅 共享日程     │
│  🔄 定时回顾         🔍 跨项目检索       👨‍👩‍👧 成长记录     │
│  📊 进度报告         ⚡ 主动提醒        🎉 活动回忆     │
└─────────────────────────────────────────────────────────┘
```

---

## 🏗️ 技术架构

```
客户端层 ──► API网关 ──► 核心服务 ──► 数据层
   │                          │
   ▼                          ▼
Web/App                   PostgreSQL  (关系数据)
Any Agent                 Qdrant      (向量数据)
                          S3/MinIO    (文件存储)
                          Redis       (缓存)
```

## 🛠️ 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| **后端** | Rust + Axum | 高性能 API |
| **前端** | React + TypeScript | Web 应用 |
| **数据库** | PostgreSQL | 关系型数据 |
| **向量库** | Qdrant | 语义搜索 |
| **缓存** | Redis | 任务队列 |
| **存储** | S3/MinIO | 文件存储 |
| **AI** | Whisper + LLM | 语音/智能 |

> 📖 详细技术决策见 [decisions/](decisions/) 目录

---

## 📖 文档

| 文档 | 说明 |
|------|------|
| [🏗️ 架构设计](docs/architecture.md) | 系统架构详解 |
| [🔌 API 文档](docs/api.md) | API 接口规范 |
| [🚀 部署指南](docs/deployment.md) | 各平台部署教程 |
| [🛠️ 开发指南](docs/development.md) | 本地开发说明 |
| [📅 路线图](docs/roadmap.md) | 开发计划 |

---

## 🤝 贡献

欢迎各种形式的贡献！

- 🐛 提交 [Issue](https://github.com/blackfaced/MemoryNexus/issues)
- 💡 提交 [Feature Request](https://github.com/blackfaced/MemoryNexus/issues)
- 📝 提交 Pull Request
- 📖 完善文档

请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解贡献指南。

---

## 📄 协议

本项目采用 [MIT License](LICENSE) 开源。

---

## 🙏 致谢

- 基于 [第二大脑](https://www.buildingasecondbrain.com/) 理念
- 使用 [Qdrant](https://qdrant.tech/) 作为向量数据库
- 使用 [FastAPI](https://fastapi.tiangolo.com/) 构建 API

---

<p align="center">
  <strong>让记忆连接，让知识生长</strong><br>
  ⭐ 如果对你有帮助，请 Star！
</p>
