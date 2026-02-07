# openat

Rust 实现的超轻量级个人 AI 助手

```
    |\__/,|   (`
  _.|o o  |_   )
 -(((---(((--------
```

## 功能特性

- **多模型支持**: OpenRouter、Anthropic Claude、OpenAI、Groq、Gemini、MiniMax
- **多渠道支持**: Telegram、WhatsApp、QQ（通过 OneBot）
- **工具集成**: 文件操作、shell 命令、网页搜索/抓取
- **记忆系统**: 长期记忆和会话记忆
- **定时任务**: 支持 Cron 表达式和消息推送
- **会话管理**: JSON 格式的对话历史

## 快速开始

### 安装

```bash
cargo build --release
```

### 初始化

```bash
cargo run -- onboard
```

### 配置

编辑 `~/.openat/config.json`:

```json
{
  "providers": {
    "minimax": {
      "api_key": "你的API密钥",
      "api_base": "https://api.minimax.chat/v1"
    }
  },
  "agents": {
    "defaults": {
      "model": "minimax/MiniMax-M2.1"
    }
  }
}
```

## 使用方法

### 与 Agent 对话

```bash
# 发送单条消息
cargo run -- agent "你好！"

# 交互模式
cargo run -- agent
```

### 渠道命令

```bash
# 查看渠道状态
cargo run -- channel-status

# QQ 设置帮助
cargo run -- channel-login qq
```

### 网关模式

启动所有已启用的渠道：

```bash
cargo run -- gateway
```

## 配置说明

### 模型提供商

| 提供商 | 配置键 | 必需参数 |
|--------|--------|----------|
| MiniMax | `minimax` | `api_key` |
| OpenRouter | `openrouter` | `api_key` |
| Anthropic | `anthropic` | `api_key` |
| OpenAI | `openai` | `api_key` |
| Groq | `groq` | `api_key` |
| Gemini | `gemini` | `api_key` |

### 消息渠道

#### Telegram

```json
"telegram": {
  "enabled": true,
  "token": "你的机器人Token",
  "allowed_users": ["123456789"]
}
```

#### WhatsApp

```json
"whatsapp": {
  "enabled": true,
  "bridge_url": "ws://localhost:3001"
}
```

#### QQ（通过 OneBot）

```json
"qq": {
  "enabled": true,
  "api_url": "http://localhost:3000",
  "event_url": "ws://localhost:3000",
  "access_token": "",
  "allowed_users": ["10001"]
}
```

**注意**: QQ 渠道需要配合 OneBot v11 兼容的客户端使用，推荐 [go-cqhttp](https://github.com/Mrs4s/go-cqhttp)

### 工具

Agent 可用的工具：
- `read_file`: 读取文件内容
- `write_file`: 写入文件
- `list_dir`: 列出目录内容
- `exec`: 执行 shell 命令
- `web_search`: 搜索网页
- `web_fetch`: 获取网页内容

### 定时任务

```bash
# 列出任务
cargo run -- cron-list

# 添加任务
cargo run -- cron-add "daily-check" "早上好！" --every 86400

# 删除任务
cargo run -- cron-remove <任务ID>
```

## 项目结构

```
src/
├── agent/          # Agent 实现
│   ├── memory.rs  # 记忆系统
│   ├── skills.rs  # 技能系统
│   └── tools/     # 工具定义
├── channels/       # 渠道实现
│   ├── telegram.rs
│   ├── whatsapp.rs
│   └── qq.rs      # QQ（OneBot）
├── cli.rs         # CLI 命令
├── config/        # 配置模块
├── cron.rs        # 任务调度
├── heartbeat.rs   # 心跳监控
├── providers.rs   # LLM 提供商
├── session.rs     # 会话管理
└── bus.rs         # 消息总线
```

## 架构设计

### 核心组件

```
┌─────────────────────────────────────────────────────┐
│                    CLI / Gateway                       │
├─────────────────────────────────────────────────────┤
│                      Agent                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Memory    │  │   Skills    │  │   Tools     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                   Message Bus                        │
├───────────────┬───────────────┬───────────────────────┤
│   Telegram    │   WhatsApp   │         QQ            │
│   Channel     │   Channel    │      (OneBot)        │
└───────────────┴───────────────┴───────────────────────┘
```

### 模块说明

- **Agent**: 核心对话引擎，处理用户输入、工具调用、记忆管理
- **Memory**: 长期记忆（持久化）和短期记忆（会话内）
- **Skills**: 可插拔的技能系统
- **Tools**: 文件操作、命令执行、网页搜索等
- **Message Bus**: 解耦的消息总线，支持多渠道统一处理
- **Channels**: 适配不同消息平台（Telegram/WhatsApp/QQ）
- **Cron**: 定时任务调度系统
- **Providers**: 多模型提供商抽象层

## 技术选型

### 核心框架

| 组件 | 技术 | 选型理由 |
|------|------|----------|
| 运行时 | Tokio | Rust 异步标准，性能优异 |
| CLI | Clap | 功能完整的命令行解析 |
| 配置 | serde_json | JSON 格式简单易用 |
| 日志 | tracing | 现代化日志框架 |

### LLM 集成

| 提供商 | 协议 | 特点 |
|--------|------|------|
| MiniMax | OpenAI 兼容 API | 国内访问快 |
| OpenRouter | OpenAI 兼容 API | 聚合多模型 |
| Anthropic | 独立 API | Claude 系列 |
| OpenAI | OpenAI 兼容 API | GPT 系列 |
| Groq | OpenAI 兼容 API | 推理速度快 |
| Gemini | 独立 API | Google 最新模型 |

### 消息渠道

| 渠道 | 协议 | 说明 |
|------|------|------|
| Telegram | Bot API | 官方 Bot API |
| WhatsApp | WebSocket | WA Bridge 中转 |
| QQ | OneBot v11 | go-cqhttp 等 |

### 存储

| 类型 | 实现 | 用途 |
|------|------|------|
| 配置 | JSON 文件 | 用户配置持久化 |
| 记忆 | Markdown 文件 | 长期记忆 |
| 会话 | JSONL 文件 | 对话历史 |
| 任务 | JSON 文件 | Cron 任务 |

## 扩展开发

### 添加新渠道

1. 在 `src/channels/` 下创建新文件
2. 实现 `start_channel()` 函数
3. 在 `channels/mod.rs` 中导出
4. 在 `config/mod.rs` 中添加配置结构

### 添加新工具

1. 在 `src/agent/tools/` 中定义工具
2. 实现工具逻辑
3. 在 `get_tool_definitions()` 中注册

### 添加新提供商

1. 在 `src/providers.rs` 中实现 `LLMProvider` trait
2. 在 `create_provider()` 中添加分支

## 许可证

MIT
