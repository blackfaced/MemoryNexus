# ADR-005: 项目命名选择

## 状态
✅ 已接受

> 当前品牌语义由 ADR-022 补充：`MemoryNexus` 名称保留，但不再表示家庭照片、
> second brain 或 generic memory app。`Memory` 应理解为可演化的长期 Trace、
> FeedbackLoop、GrowthModel 和下一步行动上下文。

## 背景
ADR-005 记录项目早期命名选择。当时项目仍偏向家庭 AI 记忆中心和第二大脑方向。
当前项目定位已经由 ADR-018 和 ADR-022 更新为本地优先的长期反馈引擎；本 ADR
保留历史命名决策，不再作为当前产品定位说明。

## 候选方案

| 名称 | 优点 | 缺点 | 适合度 |
|------|------|------|--------|
| **MemoryNexus** | 国际化、好记、域名可注册 | 无明显缺点 | ⭐⭐⭐⭐⭐ |
| BrainVault | 安全感、信任感 | 偏金融/安全 | ⭐⭐⭐⭐ |
| 忆联 | 中文好记、有深意 | 国际化程度低 | ⭐⭐⭐⭐ |
| FamilyBrain | 目标清晰 | 俗套 | ⭐⭐⭐ |
| 心智库 | 有文化底蕴 | 偏学术 | ⭐⭐⭐ |

## 决策

### 选择：MemoryNexus

**含义：**
- Memory = 记忆
- Nexus = 枢纽、连接点
- 整体含义：记忆的连接点

**契合度：**
- ✅ 长期 Trace / FeedbackLoop / GrowthModel → Memory
- ✅ 复盘、整合、下一步行动 → Nexus
- ✅ 开源项目 → GitHub 地址好看

**品牌优势：**
- `blackfaced/MemoryNexus` - GitHub 地址
- memorynexus.com - 可注册域名
- 国际化命名，海外推广友好

## 后果

**正面：**
- 品牌定位清晰
- 易于国际化
- 域名和社交媒体账号容易获取

**负面：**
- 中文名不够响亮（需另起中文名）
- `Memory` 一词容易被误解为家庭照片、视频、second brain 或 agent memory store；
  当前解释以 ADR-022 为准。
