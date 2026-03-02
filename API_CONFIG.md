# API 配置说明

## DashScope（阿里云）配置

### 1. 获取 API Key
访问：https://dashscope.console.aliyun.com/apiKey

### 2. 设置环境变量（永久）
```powershell
setx OPENAI_API_KEY "sk-你的实际 API Key"
setx OPENAI_BASE_URL "https://dashscope.aliyuncs.com/compatible-mode/v1

# 或者使用 DashScope 旧版兼容端点（如遇到问题可尝试）
# https://coding.dashscope.aliyuncs.com/v1"
```
**注意：** 设置后需重启 PowerShell 才能生效

### 3. 临时使用（当前会话）
```powershell
$env:OPENAI_API_KEY = "sk-你的实际 API Key"
$env:OPENAI_BASE_URL = "https://dashscope.aliyuncs.com/compatible-mode/v1

# 或者使用 DashScope 旧版兼容端点（如遇到问题可尝试）
# https://coding.dashscope.aliyuncs.com/v1"
```

### 4. 命令行参数（优先级最高）
```powershell
py ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,RAG --ai `
  --api-key "sk-你的实际 API Key" `
  --base-url "https://dashscope.aliyuncs.com/compatible-mode/v1

# 或者使用 DashScope 旧版兼容端点（如遇到问题可尝试）
# https://coding.dashscope.aliyuncs.com/v1" `
  --model "qwen3.5-plus"
```

## 默认配置

| 参数 | 默认值 |
|------|--------|
| Base URL | https://dashscope.aliyuncs.com/compatible-mode/v1

# 或者使用 DashScope 旧版兼容端点（如遇到问题可尝试）
# https://coding.dashscope.aliyuncs.com/v1 |
| Model | qwen3.5-plus |
| API Key | 环境变量 OPENAI_API_KEY |

## 验证配置

```powershell
# 检查环境变量
echo $env:OPENAI_API_KEY
echo $env:OPENAI_BASE_URL

# 测试调用
py ai_research_os.py https://arxiv.org/abs/2601.00155 --tags Test --ai
```

----------------------------------------------------------------------------------

py ai_research_os.py https://arxiv.org/abs/2602.23958
会做的事是：拉 arXiv 元数据 + 下载 PDF + 抽取文本 + 生成 P-Note/C-Note/Radar/Timeline；**AI 自动初稿不会生成**。

如果你想让它写 “AI 自动初稿（待核验）”，要这样跑：
py ai_research_os.py https://arxiv.org/abs/2602.23958 --ai

如果还想带标签：
py ai_research_os.py https://arxiv.org/abs/2602.23958 --tags Audio,TTS,Evaluation --ai
