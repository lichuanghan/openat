# openat

**Rust Personal AI Assistant** | For Discord, QQ

```
    |__|   OpenAT
   / o o \
  (  ^  )
   \_____/
```

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## What is openat?

**openat** is an ultra-lightweight, high-performance **personal AI assistant** written in **Rust**.

## Features

### Multi-Platform Integration
- **Discord** - Bot API with Gateway/WebSocket support
- **QQ** - Official Bot API (WebSocket + REST)

### Multi-LLM Provider Support
- **MiniMax** (M2.1)
- **OpenAI** (GPT-4o, GPT-4o-mini)
- **Anthropic** (Claude 3.5 Sonnet)
- **OpenRouter** (Unified access to 100+ models)
- **Groq** (Fast inference)
- **Google Gemini**

### Agent Capabilities
- **Tool execution** - Web search
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
cargo build --release -p openat-cli
```

### Configuration

Edit `~/.openat/config.json`:

```json
{
  "model": "MiniMax-M2.1",
  "providers": {
    "openrouter": {
      "api_key": "your-api-key"
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

### Run

```bash
# Start the gateway
cargo run -p openat-cli -- gateway

# Or with Docker
docker-compose up -d
```

### HTTP API

```bash
# Health check
curl http://localhost:18790/health

# Send message
curl -X POST http://localhost:18790/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"hello","channel":"test","user":"u1","chat_id":"c1"}'
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Gateway (HTTP Server)              │
├─────────────────────────────────────────────────────┤
│                    AgentExecutor                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │  Provider   │  │   Tools    │  │   Memory    │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────┤
│                   MessageBus                         │
├───────────────┬───────────────┬─────────────────────┤
│   Discord    │      QQ      │       ...           │
│   Gateway    │   Official   │                      │
├───────────────┴───────────────┴─────────────────────┤
│           LLM Providers (MiniMax, OpenAI...)        │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
openat/
├── Cargo.toml                 # Workspace
├── Dockerfile                # Docker build
├── docker-compose.yml         # Docker Compose
│
└── crates/
    ├── openat-cli/          # CLI entry point
    ├── openat-agent/        # Agent executor
    ├── openat-channels/     # Channel adapters (Discord, QQ)
    ├── openat-config/      # Configuration
    ├── openat-runtime/     # Message bus
    ├── openat-types/       # Shared types
    ├── openat-providers/   # LLM providers
    └── openat-tools/       # Tools (web search)
```

## Docker Deployment

```bash
# Build and run
docker-compose up -d

# Check logs
docker-compose logs -f

# Stop
docker-compose down
```

## License

MIT License

---

**Built with ❤️ in Rust**
