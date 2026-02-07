# openat

Rust 实现的超轻量级个人 AI 助手

```
    |\__/,|   (`
  _.|o o  |_   )
 -(((---(((--------
```

## 功能特性

- **多模型支持**: MiniMax、DeepSeek、OpenRouter、Anthropic、OpenAI、Groq、Gemini、Zhipu、Moonshot、VLLM
- **多渠道支持**: Telegram、WhatsApp、QQ（OneBot）、Discord、Feishu
- **工具集成**: 文件操作、shell 命令、网页搜索/抓取、定时任务
- **记忆系统**: 长期记忆和会话记忆
- **Agent**: 简单 CLI Agent 和完整功能执行器

## 快速开始

### 安装

```bash
cargo build --release
```

或使用预编译的二进制文件。

### 初始化

```bash
./target/release/openat onboard
```

### 配置

编辑 `~/.openat/config.json`:

```json
{
  "providers": {
    "minimax": {
      "api_key": "你的API密钥",
      "api_base": "https://api.minimax.chat/v1"
    },
    "deepseek": {
      "api_key": "你的API密钥",
      "api_base": null
    }
  },
  "agents": {
    "defaults": {
      "model": "minimax/MiniMax-M2.1",
      "max_tokens": 4096,
      "temperature": 0.7
    }
  }
}
```

## 使用方法

### 与 Agent 对话

```bash
# 发送单条消息
openat agent "你好！"

# 交互模式
openat agent
```

### CLI 命令

```bash
# 查看状态
openat status

# 渠道管理
openat channel-status
openat channel-login telegram
openat channel-login qq

# 网关模式（运行所有已启用的渠道）
openat gateway
```

## 支持的提供商

| 提供商 | 环境变量 | 配置键 | 状态 |
|--------|----------|--------|------|
| MiniMax | `MINIMAX_API_KEY` | `minimax` | ✅ 已验证 |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` | ✅ 已验证 |
| OpenRouter | `OPENROUTER_API_KEY` | `openrouter` | 已就绪 |
| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` | 已就绪 |
| OpenAI | `OPENAI_API_KEY` | `openai` | 已就绪 |
| Groq | `GROQ_API_KEY` | `groq` | 已就绪 |
| Gemini | `GEMINI_API_KEY` | `gemini` | 已就绪 |
| Zhipu | `ZHIPU_API_KEY` | `zhipu` | 已就绪 |
| Moonshot | `MOONSHOT_API_KEY` | `moonshot` | 已就绪 |
| VLLM | - | `vllm` | 已就绪 |

## 支持的渠道

| 渠道 | 协议 | 状态 |
|------|------|------|
| Telegram | Bot API | 已就绪 |
| WhatsApp | WebSocket Bridge | 已就绪 |
| QQ | OneBot v11 | 已就绪 |
| Discord | Bot API | 已就绪 |
| Feishu | App Webhook | 已就绪 |

## 可用工具

| 工具 | 说明 |
|------|------|
| `read_file` | 读取文件内容 |
| `write_file` | 写入文件到磁盘 |
| `list_dir` | 列出目录内容 |
| `exec` | 执行 shell 命令 |
| `web_search` | 搜索网页 |
| `web_fetch` | 获取网页内容 |

## 项目结构

```
src/
├── main.rs                 # 入口点
├── cli/                    # CLI 命令
│   ├── mod.rs
│   └── commands/
│       ├── agent.rs        # Agent 命令
│       ├── channel.rs      # 渠道管理
│       ├── cron.rs         # 定时任务
│       └── gateway.rs       # 网关模式
├── core/                   # 核心模块
│   ├── agent/              # Agent 实现
│   │   ├── simple.rs       # 简单 CLI agent
│   │   ├── executor.rs     # 完整 agent（含工具）
│   │   ├── memory.rs       # 记忆管理
│   │   ├── skills.rs       # 技能系统
│   │   └── context.rs      # 上下文构建器
│   ├── bus/                # 消息总线
│   ├── session/            # 会话管理
│   └── scheduler/          # 定时调度器
├── llm/                    # LLM 提供商
│   ├── providers/          # 提供商实现
│   │   ├── minimax.rs
│   │   ├── deepseek.rs
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   └── ...
│   └── mod.rs
├── channels/               # 渠道适配器
│   ├── telegram/
│   ├── whatsapp/
│   ├── qq/
│   ├── discord/
│   └── feishu.rs
├── tools/                  # 工具实现
│   ├── filesystem.rs       # 文件操作
│   ├── shell.rs             # Shell 命令
│   ├── web_search.rs       # 网页搜索
│   ├── fetch.rs            # URL 获取
│   └── cron_tool.rs        # 定时任务
├── config/                 # 配置模块
├── types/                  # 类型定义
└── heartbeat/              # 心跳监控
```

## 代码统计

- **源文件**: 50 个 Rust 文件
- **总行数**: 8,309 行
- **测试覆盖**: 57 个单元测试通过

## 测试

```bash
# 运行所有测试
cargo test

# 库测试
cargo test --lib

# E2E 测试
openat agent "你的消息"
```

## 配置指南

### 环境变量

可以使用环境变量代替配置文件：

```bash
export MINIMAX_API_KEY="你的密钥"
export DEEPSEEK_API_KEY="你的密钥"
```

### 优先级顺序

当配置了多个提供商时：
1. 环境变量（优先检查）
2. 配置文件值（按优先级顺序）

提供商优先级：OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax > DeepSeek > Zhipu > Moonshot

## Docker

```bash
# 构建
docker build -t openat .

# 运行
docker run -d \
  -e MINIMAX_API_KEY="你的密钥" \
  -v ~/.openat:/home/openat/.openat \
  -p 18790:18790 \
  openat
```

或使用 docker-compose：

```bash
docker-compose up -d
```

## 架构

```
┌─────────────────────────────────────────────────────┐
│                    CLI / 网关                          │
├─────────────────────────────────────────────────────┤
│                       Agent                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │    记忆     │  │   技能    │  │   工具      │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                    消息总线                             │
├───────────────┬───────────────┬───────────────────────┤
│    Telegram   │   WhatsApp   │         QQ            │
│    渠道       │   渠道       │      (OneBot)         │
├───────────────┴───────────────┴───────────────────────┤
│                    LLM 提供商                          │
│  MiniMax | DeepSeek | OpenAI | Anthropic | ...    │
└─────────────────────────────────────────────────────┘
```

## 许可证

MIT
