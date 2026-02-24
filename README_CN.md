# openat

**Rust 个人 AI 助手** | 支持 Discord、QQ

```
    |__|   OpenAT
   / o o \
  (  ^  )
   \_____/
```

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## 什么是 openat？

**openat** 是一个用 **Rust** 编写的超轻量级、高性能 **个人 AI 助手**。

## 功能特性

### 多平台集成
- **Discord** - Bot API + Gateway/WebSocket 支持
- **QQ** - 官方机器人 API (WebSocket + REST)

### 多模型提供商支持
- **MiniMax** (M2.1)
- **OpenAI** (GPT-4o, GPT-4o-mini)
- **Anthropic** (Claude 3.5 Sonnet)
- **OpenRouter** (统一访问 100+ 模型)
- **Groq** (快速推理)
- **Google Gemini**

### Agent 能力
- **工具执行** - 网页搜索
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
cargo build --release -p openat-cli
```

### 配置

编辑 `~/.openat/config.json`：

```json
{
  "model": "MiniMax-M2.1",
  "providers": {
    "openrouter": {
      "api_key": "你的API密钥"
    }
  },
  "channels": {
    "discord": {
      "enabled": true,
      "token": "Bot xxx",
      "allowed_users": []
    },
    "qq": {
      "enabled": false,
      "app_id": "xxx",
      "client_secret": "xxx",
      "sandbox": true,
      "listen_group": true,
      "listen_private": true
    }
  }
}
```

### 运行

```bash
# 启动网关
cargo run -p openat-cli -- gateway

# 或使用 Docker
docker-compose up -d
```

### HTTP API

```bash
# 健康检查
curl http://localhost:18790/health

# 发送消息
curl -X POST http://localhost:18790/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"你好","channel":"test","user":"u1","chat_id":"c1"}'
```

## 架构

```
┌─────────────────────────────────────────────────────┐
│                    网关 (HTTP 服务器)                   │
├─────────────────────────────────────────────────────┤
│                   AgentExecutor                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│
│  │   提供商    │  │   工具     │  │    记忆    ││
│  └─────────────┘  └─────────────┘  └─────────────┘│
├─────────────────────────────────────────────────────┤
│                   MessageBus                         │
├───────────────┬───────────────┬─────────────────────┤
│    Discord   │      QQ      │       ...           │
│    Gateway   │   官方API    │                      │
├───────────────┴───────────────┴─────────────────────┤
│           LLM 提供商 (MiniMax, OpenAI...)          │
└─────────────────────────────────────────────────────┘
```

## 项目结构

```
openat/
├── Cargo.toml               # 工作空间
├── Dockerfile              # Docker 构建
├── docker-compose.yml       # Docker Compose
│
└── crates/
    ├── openat-cli/        # CLI 入口
    ├── openat-agent/       # Agent 执行器
    ├── openat-channels/    # 渠道适配器 (Discord, QQ)
    ├── openat-config/      # 配置管理
    ├── openat-runtime/     # 消息总线
    ├── openat-types/       # 共享类型
    ├── openat-providers/   # LLM 提供商
    └── openat-tools/       # 工具 (网页搜索)
```

## Docker 部署

```bash
# 构建并运行
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止
docker-compose down
```

## 许可证

MIT License

---

**用 ❤️ 在 Rust 中构建**
