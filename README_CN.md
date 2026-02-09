# openat

**Rust 个人 AI 助手** | 灵感来自 nanobot 和 openclawd | 支持 Discord、Telegram、QQ、WhatsApp

```
        ()-()
      .-(___)-.
       _<   >_
       \/   \/
```

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## 什么是 openat？

**openat** 是一个用 **Rust** 编写的超轻量级、高性能 **个人 AI 助手**。灵感来源于 **nanobot** 和 **openclawd**，专为希望在 **社交平台**（Discord、Telegram、QQ 等）上运行自己 **AI 机器人**的个人和社区设计。

本项目借鉴了：
- **nanobot** - 极简机器人框架理念
- **openclawd** - OpenCLAWD 协议和自动化模式

适用于：
- 在 Discord/Telegram/QQ 上部署你的 **个人 AI 助手**
- 构建 **nanobot** 风格的轻量级机器人
- **OpenCLAWD** 兼容的自动化工作流
- **社交媒体** AI 助手开发
- 通过机器人开发学习 **Rust** 异步编程

## 功能特性

### 多平台集成
- **Discord** - Bot API + Gateway/WebSocket 支持
- **Telegram** - Bot API 集成
- **QQ** - OneBot v11 协议（Go-CQHTTP 兼容）
- **WhatsApp** - WebSocket Bridge 支持
- **Feishu** - 飞书 Webhook 集成

### 多模型提供商支持
- **MiniMax** (M2.1, Hailuo)
- **DeepSeek** (Chat, Reasoner)
- **OpenAI** (GPT-4o, GPT-4, GPT-3.5)
- **Anthropic** (Claude 3.5, Claude 3)
- **OpenRouter** (统一访问 100+ 模型)
- **Groq** (快速推理)
- **Google Gemini**
- **Moonshot** (Kimi)
- **Zhipu** (智谱AI)
- **VLLM** (自托管模型)

### Agent 能力
- **工具执行** - 文件 I/O、Shell 命令、网页搜索
- **记忆系统** - 会话持久化、长期记忆
- **定时任务** - Cron 调度自动化
- **网页工具** - Brave Search、URL 获取
- **消息总线架构** - 解耦、可扩展设计

## 快速开始

### 前置要求

- **Rust** 1.70+ (`rustup update`)
- **Git**

### 构建与安装

```bash
# 克隆仓库
git clone https://github.com/你的用户名/openat.git
cd openat

# 使用 Cargo 构建
cargo build --release

# 初始化配置
./target/release/openat onboard
```

### 配置

编辑 `~/.openat/config.json`：

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
  "channels": {
    "discord": {
      "enabled": true,
      "token": "你的Discord机器人Token",
      "allowed_users": ["你的用户ID"]
    }
  }
}
```

### 运行

```bash
# 启动网关（运行所有已启用的渠道）
./target/release/openat gateway

# 或通过 CLI 对话
./target/release/openat agent "你好，世界！"

# 交互模式
./target/release/openat agent
```

## 使用示例

### Discord 机器人

```bash
# 在 config.json 中配置后：
openat gateway
```

在 Discord 中提及你的机器人：
```
@openat帮我读取~/test.txt
@openat今天天气怎么样
@openat用中文介绍你自己
```

### CLI Agent

```bash
openat agent "用Python写一个快速排序"
openat agent "解释一下Rust的Ownership"
```

### 渠道管理

```bash
# 查看状态
openat status

# 登录/绑定渠道
openat channel-login telegram
openat channel-login qq
openat channel-login discord

# 列出定时任务
openat cron-list
```

## 架构

```
┌─────────────────────────────────────────────────────┐
│                    CLI / 网关                          │
├─────────────────────────────────────────────────────┤
│                   Agent 执行器                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │    记忆     │  │   技能     │  │   工具      │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                  消息总线 (Tokio)                       │
├───────────────┬───────────────┬───────────────────────┤
│    Discord   │   Telegram   │         QQ           │
│    Gateway   │   Bot API    │      (OneBot)        │
├───────────────┴───────────────┴───────────────────────┤
│              LLM 提供商 (MiniMax, DeepSeek...)        │
└─────────────────────────────────────────────────────┘
```

### 核心组件

| 组件 | 描述 | 技术 |
|------|------|------|
| **消息总线** | 解耦异步消息 | Tokio broadcast |
| **Agent 执行器** | 工具增强的 LLM Agent | 异步 Rust |
| **渠道适配器** | 平台集成 | WebSocket/HTTP |
| **LLM 提供商** | 模型抽象层 | OpenAI 兼容 API |

## 可用工具

| 工具 | 说明 | 示例 |
|------|------|------|
| `read_file` | 读取文件内容 | `read_file(path="~/notes.md")` |
| `write_file` | 写入文件 | `write_file(path="log.txt", content="...")` |
| `list_dir` | 目录列表 | `list_dir(path="/home")` |
| `exec` | Shell 命令 | `exec(cmd="ls -la")` |
| `web_search` | 网页搜索 | `web_search(query="Rust 2024 新闻")` |
| `web_fetch` | URL 内容 | `web_fetch(url="https://...")` |

## Docker 部署

```bash
# 构建镜像
docker build -t openat .

# 运行容器
docker run -d \
  -e MINIMAX_API_KEY="你的密钥" \
  -e DEEPSEEK_API_KEY="你的密钥" \
  -v ~/.openat:/home/openat/.openat \
  -p 18790:18790 \
  openat
```

或使用 docker-compose：
```bash
docker-compose up -d
```

## 项目结构

```
openat/
├── src/
│   ├── main.rs              # 入口点
│   ├── cli/                  # CLI 命令 (Clap)
│   │   ├── agent.rs         # Agent 交互
│   │   ├── gateway.rs       # 网关模式
│   │   ├── cron.rs          # Cron 管理
│   │   └── commands/        # 命令模块
│   ├── core/                 # 核心框架
│   │   ├── agent/           # Agent 逻辑
│   │   ├── bus/             # 消息总线
│   │   ├── scheduler/       # 任务调度
│   │   └── session/         # 会话管理
│   ├── channels/             # 平台适配器
│   │   ├── discord/        # Discord 机器人
│   │   ├── telegram/        # Telegram 机器人
│   │   ├── qq/              # OneBot/QQ
│   │   ├── whatsapp/        # WhatsApp 桥接
│   │   └── feishu/          # 飞书/Lark
│   ├── llm/                  # LLM 提供商
│   │   ├── providers/       # 提供商实现
│   │   │   ├── minimax.rs  # MiniMax
│   │   │   ├── deepseek.rs # DeepSeek
│   │   │   ├── openai.rs   # OpenAI
│   │   │   └── anthropic.rs # Anthropic
│   │   └── mod.rs
│   ├── tools/                # 工具系统
│   │   ├── filesystem.rs   # 文件 I/O
│   │   ├── shell.rs        # Shell 执行
│   │   ├── web_search.rs   # 网页搜索
│   │   └── cron_tool.rs     # Cron 任务
│   ├── config/               # 配置
│   ├── types/                # 类型定义
│   └── heartbeat/            # 健康监控
├── Cargo.toml               # Rust 包清单
├── Dockerfile               # 容器构建
└── docker-compose.yml      # 编排配置
```

## 代码统计

- **语言**: Rust (async/await, Tokio 运行时)
- **源文件**: 50+ Rust 模块
- **代码行数**: ~8,300
- **测试**: 57+ 单元测试
- **依赖**: Tokio, Clap, reqwest, serde, tracing

## 相关项目

- **nanobot** - 启发本项目的极简机器人理念
- **openclawd** - OpenCLAWD 自动化协议和模式
- **OneBot** - QQ 机器人协议标准 (go-cqhttp)
- **OpenAI** - API 设计参考
- **Anthropic** - Claude 集成模式参考

## 许可证

MIT License - 自由使用、修改和分发。

## 贡献

欢迎提交 PR！本项目对学习和fork友好：
- **Rust** 初学者探索异步编程
- **机器人** 开发者构建社交集成
- **AI** 爱好者实验 LLM Agent
- **自动化** 工程师创建工作流

---

**用 ❤️ 在 Rust 中构建** | 个人 AI | nanobot 启发 | openclawd 就绪 | 轻量 | 快速 | 可靠
