# ADR-004: Whisper 语音识别方案

## 状态
✅ 已接受

## 背景
需要语音转文字功能，支持本地和云端部署，评估 GPU 需求。

## 决策

### 选择：双模式支持

```rust
pub trait WhisperProvider: Send + Sync {
    async fn transcribe(&self, audio_data: Vec<u8>) -> Result<TranscriptionResult>;
}

#[derive(Debug, Clone, Enum)]
pub enum WhisperMode {
    OpenAI(OpenAIWhisper),  // 云端
    Local(LocalWhisper),    // 本地 whisper.cpp
}
```

### 方案对比

| 特性 | OpenAI API | whisper.cpp 本地 |
|------|-----------|-----------------|
| 配置复杂度 | ✅ 零配置 | ⚠️ 需要模型文件 |
| GPU 需求 | ❌ 无 | ⚠️ 推荐 GPU |
| 成本 | ⚠️ 按调用计费 | ✅ 免费 |
| 隐私 | ⚠️ 数据上云 | ✅ 完全本地 |
| 速度 (1小时音频) | ~2分钟 | CPU: ~6分钟, GPU: ~36秒 |

### GPU 需求说明

```
whisper.cpp 性能基准：
─────────────────────────────────
CPU (4核):     ~10x 实时  (1小时 → 6分钟)
GPU (NVIDIA):  ~100x 实时 (1小时 → 36秒)
Apple M1+:     ~50x 实时  (1小时 → 1.2分钟)
─────────────────────────────────
```

**建议：**
- 开发阶段：使用 OpenAI API
- 生产环境：自建 whisper.cpp 服务 + GPU
- 完全离线：本地 whisper.cpp + CPU

### 配置示例

```yaml
whisper:
  provider: "openai"  # 开发环境
  # provider: "local"  # 生产环境

openai_whisper:
  api_key: "${OPENAI_API_KEY}"
  model: "whisper-1"
  
local_whisper:
  model_path: "./models/ggml-base.bin"
  use_gpu: true
  n_threads: 4
```

## 后果

**正面：**
- 开发阶段快速上手（OpenAI API）
- 生产环境可控成本（whisper.cpp）
- 数据隐私可选（完全本地）

**负面：**
- 两套实现需要维护
- 本地 GPU 部署有运维成本

## 相关决策
- ADR-001: Rust 后端选择
