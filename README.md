# openat

**Rust Personal AI Assistant** | Inspired by nanobot & openclawd | For Discord, Telegram, QQ, WhatsApp

```
        ()-()
      .-(___)-.
       _<   >_
       \/   \/
```

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## What is openat?

**openat** is an ultra-lightweight, high-performance **personal AI assistant** written in **Rust**. Inspired by **nanobot** and **openclawd**, it's designed for individuals and communities who want their own **AI bot** running on **social platforms**.

This project draws inspiration from:
- **nanobot** - Minimalist bot framework philosophy
- **openclawd** - OpenCLAWD protocol and automation patterns

Perfect for:
- Running your **personal AI assistant** on Discord/Telegram/QQ
- Building **nanobot**-style lightweight, focused bots
- **OpenCLAWD** compatible automation workflows
- **Social media** AI assistant development
- Learning **Rust** async programming through bot development

## Features

### Multi-Platform Integration
- **Discord** - Bot API with Gateway/WebSocket support
- **Telegram** - Bot API integration
- **QQ** - OneBot v11 protocol (Go-CQHTTP compatible)
- **WhatsApp** - WebSocket bridge support
- **Feishu** - Lark webhook integration

### Multi-LLM Provider Support
- **MiniMax** (M2.1, Hailuo)
- **DeepSeek** (Chat, Reasoner)
- **OpenAI** (GPT-4o, GPT-4, GPT-3.5)
- **Anthropic** (Claude 3.5, Claude 3)
- **OpenRouter** (Unified access to 100+ models)
- **Groq** (Fast inference)
- **Google Gemini**
- **Moonshot** (Kimi)
- **Zhipu** (智谱AI)
- **VLLM** (Self-hosted models)

### Agent Capabilities
- **Tool execution** - File I/O, shell commands, web search
- **Memory system** - Session persistence, long-term memory
- **Cron scheduling** - Automated tasks
- **Web tools** - Brave Search, URL fetching
- **Message bus architecture** - Decoupled, scalable design

## Quick Start

### Prerequisites

- **Rust** 1.70+ (`rustup update`)
- **Git**

### Build & Install

```bash
# Clone the repository
git clone https://github.com/yourusername/openat.git
cd openat

# Build with Cargo
cargo build --release

# Initialize configuration
./target/release/openat onboard
```

### Configuration

Edit `~/.openat/config.json`:

```json
{
  "providers": {
    "minimax": {
      "api_key": "your-api-key",
      "api_base": "https://api.minimax.chat/v1"
    },
    "deepseek": {
      "api_key": "your-api-key",
      "api_base": null
    }
  },
  "channels": {
    "discord": {
      "enabled": true,
      "token": "your-discord-bot-token",
      "allowed_users": ["your-user-id"]
    }
  }
}
```

### Run

```bash
# Start the gateway (runs all enabled channels)
./target/release/openat gateway

# Or chat via CLI
./target/release/openat agent "Hello, world!"

# Interactive mode
./target/release/openat agent
```

## Usage Examples

### Discord Bot

```bash
# Configure in config.json, then:
openat gateway
```

Mention your bot in Discord:
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

### Channel Management

```bash
# Check status
openat status

# Login/link channels
openat channel-login telegram
openat channel-login qq
openat channel-login discord

# List scheduled jobs
openat cron-list
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CLI / Gateway                       │
├─────────────────────────────────────────────────────┤
│                      Agent Executor                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Memory    │  │   Skills    │  │   Tools     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                   Message Bus (Tokio)                   │
├───────────────┬───────────────┬───────────────────────┤
│   Discord    │   Telegram    │         QQ           │
│   Gateway    │   Bot API     │      (OneBot)        │
├───────────────┴───────────────┴───────────────────────┤
│              LLM Providers (MiniMax, DeepSeek...)      │
└─────────────────────────────────────────────────────┘
```

### Key Components

| Component | Description | Technology |
|-----------|-------------|------------|
| **Message Bus** | Decoupled async messaging | Tokio broadcast |
| **Agent Executor** | Tool-augmented LLM agent | Async Rust |
| **Channel Adapters** | Platform integrations | WebSocket/HTTP |
| **LLM Providers** | Model abstraction layer | OpenAI-compatible API |

## Available Tools

| Tool | Description | Example |
|------|-------------|---------|
| `read_file` | Read file contents | `read_file(path="~/notes.md")` |
| `write_file` | Write to files | `write_file(path="log.txt", content="...")` |
| `list_dir` | Directory listing | `list_dir(path="/home")` |
| `exec` | Shell commands | `exec(cmd="ls -la")` |
| `web_search` | Web search | `web_search(query="Rust 2024 news")` |
| `web_fetch` | URL content | `web_fetch(url="https://...")` |

## Docker Deployment

```bash
# Build image
docker build -t openat .

# Run container
docker run -d \
  -e MINIMAX_API_KEY="your-key" \
  -e DEEPSEEK_API_KEY="your-key" \
  -v ~/.openat:/home/openat/.openat \
  -p 18790:18790 \
  openat
```

Or use docker-compose:
```bash
docker-compose up -d
```

## Project Structure

```
openat/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli/                  # CLI commands (Clap)
│   │   ├── agent.rs         # Agent interactions
│   │   ├── gateway.rs       # Gateway mode
│   │   ├── cron.rs          # Cron management
│   │   └── commands/        # Command modules
│   ├── core/                 # Core framework
│   │   ├── agent/           # Agent logic
│   │   ├── bus/             # Message bus
│   │   ├── scheduler/       # Task scheduling
│   │   └── session/         # Session management
│   ├── channels/             # Platform adapters
│   │   ├── discord/         # Discord bot
│   │   ├── telegram/        # Telegram bot
│   │   ├── qq/              # OneBot/QQ
│   │   ├── whatsapp/        # WhatsApp bridge
│   │   └── feishu/          # Feishu/Lark
│   ├── llm/                  # LLM providers
│   │   ├── providers/       # Provider implementations
│   │   │   ├── minimax.rs   # MiniMax
│   │   │   ├── deepseek.rs  # DeepSeek
│   │   │   ├── openai.rs    # OpenAI
│   │   │   └── anthropic.rs # Anthropic
│   │   └── mod.rs
│   ├── tools/                # Tool system
│   │   ├── filesystem.rs    # File I/O
│   │   ├── shell.rs         # Shell execution
│   │   ├── web_search.rs   # Web search
│   │   └── cron_tool.rs     # Cron tasks
│   ├── config/               # Configuration
│   ├── types/                # Type definitions
│   └── heartbeat/            # Health monitoring
├── Cargo.toml               # Rust package manifest
├── Dockerfile               # Container build
└── docker-compose.yml      # Orchestration
```

## Code Statistics

- **Language**: Rust (async/await, Tokio runtime)
- **Source Files**: 50+ Rust modules
- **Lines of Code**: ~8,300
- **Tests**: 57+ unit tests
- **Dependencies**: Tokio, Clap, reqwest, serde, tracing

## Related & Inspired By

- **nanobot** - The minimalist bot philosophy that inspired this project
- **openclawd** - OpenCLAWD automation protocols and patterns
- **OneBot** - QQ bot protocol standard (go-cqhttp)
- **OpenAI** - API design inspiration
- **Anthropic** - Claude integration patterns

## License

MIT License - feel free to use, modify, and distribute.

## Contributing

PRs welcome! This is a learning/fork-friendly project for:
- **Rust** beginners exploring async programming
- **Bot** developers building social integrations
- **AI** enthusiasts experimenting with LLM agents
- **Automation** engineers creating workflows

---

**Built with ❤️ in Rust** | Personal AI | nanobot inspired | openclawd ready |Lightweight | Fast | Reliable
