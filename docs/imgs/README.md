# 📸 MemoryNexus 图片资源

> 给人类看的可视化展示图片

## 目录

| 文件 | 用途 | 格式 |
|------|------|------|
| `memorynexus_overview.png` | 项目概览 Infographic | PNG 2K |
| `memorynexus_rust_stack.png` | Rust 技术栈全景 | PNG 2K |
| `memorynexus_status.png` | 项目现状与计划 | PNG 2K |
| `memorynexus_todo.png` | 开发路线图 | PNG 2K |
| `memorynexus_cli_design.png` | CLI + Agent 设计 | PNG 2K |
| `adr_management_tools.png` | ADR 管理工具 | PNG 2K |

## 发布

这些图片已上传 CDN，可直接引用：

```markdown
![概览](https://cdn.hailuoai.com/mcp/cdn_upload/.../memorynexus_overview.png)
```

## 生成新图片

使用 `mcp_matrix_image_synthesize` 生成：

```python
mcp_matrix_image_synthesize({
    "requests": [{
        "prompt": "描述...",
        "output_file": "/app/MemoryNexus/docs/imgs/xxx.png",
        "aspect_ratio": "16:9",
        "resolution": "2K"
    }]
})
```
