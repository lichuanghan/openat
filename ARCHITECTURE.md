# OpenAT 架构文档

> Ultra-Lightweight Personal AI Assistant

## 一、项目概述

OpenAT 是一个轻量级的个人 AI 助手框架，支持多种消息渠道（Discord、QQ 等）和多个 LLM 提供商。

## 二、当前目录结构

```
openat/
├── Cargo.toml                 # Workspace 配置
├── Dockerfile                # Docker 构建文件
├── docker-compose.yml        # Docker Compose 配置
│
└── crates/
    ├── openat-cli/           # CLI 入口
    │   └── src/main.rs
    │
    ├── openat-agent/         # Agent 执行器
    │   └── src/lib.rs        # AgentExecutor
    │
    ├── openat-channels/     # 渠道适配器
    │   └── src/
    │       ├── lib.rs       # Channel trait
    │       ├── discord.rs    # Discord 适配器
    │       ├── qq.rs         # QQ 官方机器人适配器
    │       └── telegram.rs   # Telegram 适配器 (stub)
    │
    ├── openat-config/       # 配置管理
    │   └── src/lib.rs       # Config 结构
    │
    ├── openat-runtime/      # 运行时核心
    │   └── src/
    │       ├── lib.rs
    │       └── bus.rs       # MessageBus 消息总线
    │
    ├── openat-types/        # 共享类型
    │   └── src/
    │       ├── lib.rs
    │       └── messages.rs   # InboundMessage, OutboundMessage
    │
    ├── openat-providers/    # LLM 提供商
    │   └── src/
    │       ├── lib.rs       # LLMProvider trait
    │       ├── openai.rs
    │       ├── anthropic.rs
    │       ├── minimax.rs
    │       └── ...
    │
    ├── openat-tools/        # 工具集
    │   └── src/
    │       ├── lib.rs
    │       └── web.rs       # Web search (Brave Search)
    │
    └── openat-common/       # 通用工具
        └── src/lib.rs
```

## 三、核心模块

### 1. MessageBus (消息总线)

```rust
// crates/openat-runtime/src/bus.rs
pub struct MessageBus {
    inbound: broadcast::Sender<Arc<InboundMessage>>,
    outbound: broadcast::Sender<Arc<OutboundMessage>>,
}
```

消息流程：
```
Channel (Discord/QQ)  ──publish_inbound()──>  MessageBus  ──subscribe_inbound()──>  Agent
Agent  ──publish_outbound()──>  MessageBus  ──subscribe_outbound()──>  Channel
```

### 2. AgentExecutor (Agent 执行器)

```rust
// crates/openat-agent/src/lib.rs
pub struct AgentExecutor {
    provider: Arc<dyn LLMProvider>,
    session_manager: SessionManager,
    system_prompt: String,
    workspace: PathBuf,
    bus: MessageBus,
    max_history_messages: usize,
    model: String,
}
```

### 3. Channel Trait (渠道接口)

```rust
// crates/openat-channels/src/lib.rs
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&mut self, bus: &MessageBus) -> anyhow::Result<()>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    fn is_enabled(&self) -> bool;
}
```

### 4. LLMProvider Trait (LLM 接口)

```rust
// crates/openat-providers/src/lib.rs
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<ChatResponse, String>;
    async fn chat_with_tools(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<ChatResponse, String>;
}
```

## 四、支持的消息渠道

### Discord

- **连接方式**: WebSocket Gateway
- **消息接收**: `MESSAGE_CREATE` 事件
- **消息发送**: REST API `POST /channels/{id}/messages`
- **配置字段**:
  ```json
  {
    "enabled": true,
    "token": "Bot xxx",
    "allowed_users": [],
    "intents": 37377
  }
  ```

### QQ (官方机器人 API)

- **连接方式**: WebSocket Gateway + REST API
- **认证方式**: App Access Token (每 100 分钟刷新)
- **消息接收**:
  - `GROUP_AT_MESSAGE_CREATE` (群聊@)
  - `C2C_MESSAGE_CREATE` (私聊)
  - `AT_MESSAGE_CREATE` (频道@)
- **消息发送**: REST API
- **配置字段**:
  ```json
  {
    "enabled": true,
    "app_id": "xxx",
    "client_secret": "xxx",
    "sandbox": true,
    "listen_group": true,
    "listen_private": true,
    "listen_guild": false
  }
  ```

## 五、Gateway 启动流程

```rust
// crates/openat-cli/src/main.rs
async fn gateway(port: u16) -> Result<()> {
    // 1. 加载配置
    let config = openat_config::Config::load();

    // 2. 创建 Provider
    let provider = create_provider(&config)?;

    // 3. 创建 MessageBus
    let bus = openat_runtime::MessageBus::new();

    // 4. 创建 AgentExecutor
    let executor = Arc::new(tokio::sync::Mutex::new(
        openat_agent::AgentExecutor::new(provider, &config, &bus)
    ));

    // 5. 启动渠道 (Discord/QQ)
    if config.channels.discord.enabled {
        let mut discord = openat_channels::DiscordChannel::new(config.channels.discord.clone());
        tokio::spawn(async move { discord.start(&bus).await });
    }

    if config.channels.qq.enabled {
        let mut qq = openat_channels::QQChannel::new(config.channels.qq.clone());
        tokio::spawn(async move { qq.start(&bus).await });
    }

    // 6. 启动 inbound 消息处理
    let executor_clone = executor.clone();
    tokio::spawn(async move {
        let mut rx = bus.subscribe_inbound();
        while let Ok(msg) = rx.recv().await {
            let mut exec = executor_clone.lock().await;
            let _ = exec.handle_message(&msg).await;
        }
    });

    // 7. 启动 HTTP 服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // ...
}
```

## 六、配置管理

配置文件位于 `~/.openat/config.json`:

```json
{
  "model": "MiniMax-M2.1",
  "providers": {
    "openrouter": { "api_key": "" },
    "openai": { "api_key": "" },
    "anthropic": { "api_key": "" },
    "minimax": { "api_key": "", "group_id": "", "api_key2": "" }
  },
  "channels": {
    "discord": { "enabled": true, "token": "Bot xxx", "allowed_users": [] },
    "qq": { "enabled": true, "app_id": "xxx", "client_secret": "xxx", "sandbox": true },
    "telegram": { "enabled": false, "token": "" }
  }
}
```

## 七、Docker 部署

### Dockerfile

```dockerfile
FROM rust:1.93-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p openat-cli

FROM debian:bookworm
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/openat-cli /usr/local/bin/openat
ENTRYPOINT ["openat"]
```

### docker-compose.yml

```yaml
services:
  openat:
    build: .
    command: gateway
    ports:
      - "18790:18790"
    volumes:
      - ${HOME}/.openat:/home/openat/.openat:ro
    environment:
      - RUST_LOG=openat=info
    restart: unless-stopped
```

## 八、扩展开发

### 添加新 LLM 提供商

1. 在 `crates/openat-providers/src/` 创建新文件 (如 `deepseek.rs`)
2. 实现 `LLMProvider` trait
3. 在 `crates/openat-cli/src/main.rs` 的 `create_provider()` 中添加支持

### 添加新渠道

1. 在 `crates/openat-channels/src/` 创建新文件
2. 实现 `Channel` trait
3. 在 gateway 启动时添加

## 九、常用命令

```bash
# 本地运行
cargo run -p openat-cli -- gateway

# Docker 运行
docker-compose up -d

# 测试
curl http://localhost:18790/health
curl -X POST http://localhost:18790/chat -d '{"message":"hello","channel":"test","user":"u1","chat_id":"c1"}'
```

---

*文档更新时间: 2026-02-22*
*项目: openat - Ultra-Lightweight Personal AI Assistant*
