# openat

Ultra-Lightweight Personal AI Assistant in Rust

```
    |\__/,|   (`
  _.|o o  |_   )
 -(((---(((--------
```

## Features

- **Multi-LLM Support**: OpenRouter, Anthropic Claude, OpenAI, Groq, Gemini, MiniMax
- **Multi-Channel**: Telegram, WhatsApp, QQ (via OneBot)
- **Tools**: File operations, shell commands, web search/fetch
- **Memory**: Long-term and session-based memory
- **Cron Jobs**: Scheduled tasks with delivery
- **Session Management**: JSON-based conversation history

## Quick Start

### Installation

```bash
cargo build --release
```

### Initialize

```bash
cargo run -- onboard
```

### Configure

Edit `~/.nanobot/config.json`:

```json
{
  "providers": {
    "minimax": {
      "api_key": "your-api-key",
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

## Usage

### Chat with Agent

```bash
# Single message
cargo run -- agent "Hello!"

# Interactive mode
cargo run -- agent
```

### Channel Commands

```bash
# Check channel status
cargo run -- channel-status

# QQ setup help
cargo run -- channel-login qq
```

### Gateway Mode

Start all enabled channels:

```bash
cargo run -- gateway
```

## Configuration

### Providers

| Provider | Config Key | Required |
|----------|-----------|----------|
| MiniMax | `minimax` | `api_key` |
| OpenRouter | `openrouter` | `api_key` |
| Anthropic | `anthropic` | `api_key` |
| OpenAI | `openai` | `api_key` |
| Groq | `groq` | `api_key` |
| Gemini | `gemini` | `api_key` |

### Channels

#### Telegram

```json
"telegram": {
  "enabled": true,
  "token": "your-bot-token",
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

#### QQ (OneBot)

```json
"qq": {
  "enabled": true,
  "api_url": "http://localhost:3000",
  "event_url": "ws://localhost:3000",
  "access_token": "",
  "allowed_users": ["10001"]
}
```

**Note**: QQ requires a OneBot v11 compatible client like [go-cqhttp](https://github.com/Mrs4s/go-cqhttp)

### Tools

Available tools for agent:
- `read_file`: Read file contents
- `write_file`: Write file to disk
- `list_dir`: List directory contents
- `exec`: Execute shell commands
- `web_search`: Search the web
- `web_fetch`: Fetch URL content

### Cron Jobs

```bash
# List jobs
cargo run -- cron-list

# Add job
cargo run -- cron-add "daily-check" "Good morning!" --every 86400

# Remove job
cargo run -- cron-remove <job-id>
```

## Project Structure

```
src/
├── agent/          # Agent implementation
│   ├── memory.rs  # Memory system
│   ├── skills.rs  # Skills system
│   └── tools/     # Tool definitions
├── channels/       # Channel implementations
│   ├── telegram.rs
│   ├── whatsapp.rs
│   └── qq.rs      # QQ via OneBot
├── cli.rs         # CLI commands
├── config/        # Configuration
├── cron.rs        # Job scheduling
├── heartbeat.rs   # Heartbeat monitor
├── providers.rs   # LLM providers
├── session.rs     # Session management
└── bus.rs         # Message bus
```

## Architecture Design

### Core Components

```
┌─────────────────────────────────────────────────────┐
│                    CLI / Gateway                       │
├─────────────────────────────────────────────────────┤
│                      Agent                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Memory    │  │   Skills   │  │   Tools     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                   Message Bus                         │
├───────────────┬───────────────┬───────────────────────┤
│   Telegram   │   WhatsApp   │         QQ            │
│   Channel    │   Channel    │      (OneBot)        │
└───────────────┴───────────────┴───────────────────────┘
```

### Module Description

- **Agent**: Core dialogue engine, handles user input, tool calls, memory management
- **Memory**: Long-term (persistent) and short-term (session) memory
- **Skills**: Pluggable skill system
- **Tools**: File operations, command execution, web search, etc.
- **Message Bus**: Decoupled message bus for unified multi-channel processing
- **Channels**: Adapters for different platforms (Telegram/WhatsApp/QQ)
- **Cron**: Scheduled task system
- **Providers**: Abstract layer for multiple LLM providers

## Technology Stack

### Core Framework

| Component | Technology | Reason |
|-----------|------------|--------|
| Runtime | Tokio | Rust async standard, excellent performance |
| CLI | Clap | Full-featured command-line parsing |
| Config | serde_json | Simple and easy JSON format |
| Logging | tracing | Modern logging framework |

### LLM Integration

| Provider | Protocol | Features |
|----------|----------|----------|
| MiniMax | OpenAI Compatible API | Fast access in China |
| OpenRouter | OpenAI Compatible API | Aggregates multiple models |
| Anthropic | Native API | Claude series |
| OpenAI | OpenAI Compatible API | GPT series |
| Groq | OpenAI Compatible API | Fast inference |
| Gemini | Native API | Google's latest models |

### Messaging Channels

| Channel | Protocol | Notes |
|---------|----------|-------|
| Telegram | Bot API | Official Bot API |
| WhatsApp | WebSocket | WA Bridge proxy |
| QQ | OneBot v11 | go-cqhttp, etc. |

### Storage

| Type | Implementation | Purpose |
|------|----------------|---------|
| Config | JSON file | User configuration persistence |
| Memory | Markdown file | Long-term memory |
| Session | JSONL file | Conversation history |
| Tasks | JSON file | Cron tasks |

## Extension Development

### Adding a New Channel

1. Create a new file in `src/channels/`
2. Implement `start_channel()` function
3. Export in `channels/mod.rs`
4. Add config struct in `config/mod.rs`

### Adding a New Tool

1. Define tool in `src/agent/tools/`
2. Implement tool logic
3. Register in `get_tool_definitions()`

### Adding a New Provider

1. Implement `LLMProvider` trait in `src/providers.rs`
2. Add branch in `create_provider()`

## License

MIT
