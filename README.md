# openat

Ultra-Lightweight Personal AI Assistant in Rust

```
    |\__/,|   (`
  _.|o o  |_   )
 -(((---(((--------
```

## Features

- **Multi-LLM Support**: MiniMax, DeepSeek, OpenRouter, Anthropic, OpenAI, Groq, Gemini, Zhipu, Moonshot, VLLM
- **Multi-Channel**: Telegram, WhatsApp, QQ (via OneBot), Discord, Feishu
- **Tools**: File operations, shell commands, web search/fetch, cron jobs
- **Memory**: Long-term and session-based memory
- **Agent**: Simple CLI agent and full-featured executor with tool support

## Quick Start

### Installation

```bash
cargo build --release
```

Or use pre-built binary from releases.

### Initialize

```bash
./target/release/openat onboard
```

### Configure

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
  "agents": {
    "defaults": {
      "model": "minimax/MiniMax-M2.1",
      "max_tokens": 4096,
      "temperature": 0.7
    }
  }
}
```

## Usage

### Chat with Agent

```bash
# Single message
openat agent "Hello!"

# Interactive mode
openat agent
```

### CLI Commands

```bash
# Check status
openat status

# Channel management
openat channel-status
openat channel-login telegram
openat channel-login qq

# Gateway mode (run all enabled channels)
openat gateway
```

## Supported Providers

| Provider | Env Variable | Config Key | Status |
|----------|--------------|------------|--------|
| MiniMax | `MINIMAX_API_KEY` | `minimax` | ✅ Verified |
| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` | ✅ Verified |
| OpenRouter | `OPENROUTER_API_KEY` | `openrouter` | Ready |
| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` | Ready |
| OpenAI | `OPENAI_API_KEY` | `openai` | Ready |
| Groq | `GROQ_API_KEY` | `groq` | Ready |
| Gemini | `GEMINI_API_KEY` | `gemini` | Ready |
| Zhipu | `ZHIPU_API_KEY` | `zhipu` | Ready |
| Moonshot | `MOONSHOT_API_KEY` | `moonshot` | Ready |
| VLLM | - | `vllm` | Ready |

## Supported Channels

| Channel | Protocol | Status |
|---------|----------|--------|
| Telegram | Bot API | Ready |
| WhatsApp | WebSocket Bridge | Ready |
| QQ | OneBot v11 | Ready |
| Discord | Bot API | Ready |
| Feishu | App Webhook | Ready |

## Available Tools

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `write_file` | Write file to disk |
| `list_dir` | List directory contents |
| `exec` | Execute shell commands |
| `web_search` | Search the web |
| `web_fetch` | Fetch URL content |

## Project Structure

```
src/
├── main.rs                 # Entry point
├── cli/                    # CLI commands
│   ├── mod.rs
│   └── commands/
│       ├── agent.rs        # Agent command
│       ├── channel.rs      # Channel management
│       ├── cron.rs         # Cron jobs
│       └── gateway.rs      # Gateway mode
├── core/                   # Core modules
│   ├── agent/              # Agent implementations
│   │   ├── simple.rs       # Simple CLI agent
│   │   ├── executor.rs     # Full agent with tools
│   │   ├── memory.rs       # Memory management
│   │   ├── skills.rs        # Skills system
│   │   └── context.rs      # Context builder
│   ├── bus/                # Message bus
│   ├── session/            # Session management
│   └── scheduler/          # Cron scheduler
├── llm/                    # LLM providers
│   ├── providers/          # Provider implementations
│   │   ├── minimax.rs
│   │   ├── deepseek.rs
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   └── ...
│   └── mod.rs
├── channels/              # Channel adapters
│   ├── telegram/
│   ├── whatsapp/
│   ├── qq/
│   ├── discord/
│   └── feishu.rs
├── tools/                 # Tool implementations
│   ├── filesystem.rs       # File operations
│   ├── shell.rs            # Shell commands
│   ├── web_search.rs       # Web search
│   ├── fetch.rs            # URL fetch
│   └── cron_tool.rs        # Cron jobs
├── config/                 # Configuration
├── types/                  # Type definitions
└── heartbeat/              # Heartbeat monitor
```

## Code Statistics

- **Source Files**: 50 Rust files
- **Total Lines**: 8,309 lines
- **Test Coverage**: 57 unit tests passing

## Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo test --lib

# E2E testing with provider
openat agent "Your message"
```

## Configuration Guide

### Environment Variables

You can use environment variables instead of config file:

```bash
export MINIMAX_API_KEY="your-key"
export DEEPSEEK_API_KEY="your-key"
```

### Priority Order

When multiple providers are configured:
1. Environment variables (checked first)
2. Config file values (in priority order)

Provider priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax > DeepSeek > Zhipu > Moonshot

## Docker

```bash
# Build
docker build -t openat .

# Run
docker run -d \
  -e MINIMAX_API_KEY="your-key" \
  -v ~/.openat:/home/openat/.openat \
  -p 18790:18790 \
  openat
```

Or use docker-compose:

```bash
docker-compose up -d
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CLI / Gateway                       │
├─────────────────────────────────────────────────────┤
│                      Agent                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Memory    │  │   Skills   │  │   Tools     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                   Message Bus                          │
├───────────────┬───────────────┬───────────────────────┤
│   Telegram    │   WhatsApp    │         QQ            │
│   Channel     │   Channel     │      (OneBot)         │
├───────────────┴───────────────┴───────────────────────┤
│                      LLM Providers                      │
│  MiniMax | DeepSeek | OpenAI | Anthropic | ...       │
└─────────────────────────────────────────────────────┘
```

## License

MIT
