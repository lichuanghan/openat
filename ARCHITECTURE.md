# Nanobot 项目优化方案

> Ultra-Lightweight Personal AI Assistant 架构重构计划

## 一、现状分析

### 当前架构问题

| 问题类型 | 具体问题 | 影响 |
|---------|---------|------|
| **循环依赖** | `providers.rs` ↔ `agent.rs` | 编译警告，长期难以维护 |
| **代码重复** | `OutboundMessage` 定义两次 | 维护困难，不一致风险 |
| **功能未完成** | Gateway、Cron、web工具 | 无法发挥完整能力 |
| **紧耦合** | CLI 包含过多业务逻辑 | 难以单元测试和扩展 |

### 当前目录结构

```
src/
├── main.rs
├── cli.rs                  # CLI入口 + 所有命令实现
├── config/mod.rs           # 配置管理
├── providers.rs             # LLM提供商 (循环依赖源)
├── bus.rs                   # 消息总线
├── cron.rs                  # 定时任务
├── session.rs               # 会话管理
├── heartbeat.rs             # 心跳监控
├── agent/
│   ├── agent.rs            # Agent核心
│   ├── tools/mod.rs        # Web搜索工具
│   ├── skills.rs           # 技能系统 (未使用)
│   └── memory.rs           # 记忆系统 (未使用)
└── channels/
    ├── mod.rs              # Channel trait
    ├── telegram.rs
    ├── whatsapp.rs
    └── qq.rs
```

---

## 二、目标架构

### 优化后的目录结构

```
src/
├── cli/                    # CLI层 - 只负责参数解析和命令路由
│   ├── mod.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── agent.rs       # agent 命令
│   │   ├── cron.rs        # cron 命令
│   │   ├── gateway.rs     # gateway 命令
│   │   └── channel.rs     # channel 命令
│   └── errors.rs          # CLI 错误类型
│
├── core/                  # 核心业务层 - 可独立测试
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── executor.rs    # Agent 执行器
│   │   ├── skills.rs      # 技能系统
│   │   └── memory.rs      # 记忆系统
│   ├── bus/               # 消息总线
│   │   └── mod.rs
│   ├── scheduler/         # 定时任务
│   │   └── mod.rs
│   └── session/           # 会话管理
│       └── mod.rs
│
├── llm/                   # LLM 提供商层
│   ├── mod.rs            # LLMProvider trait
│   ├── openrouter.rs
│   ├── anthropic.rs
│   ├── openai.rs
│   ├── groq.rs
│   ├── gemini.rs
│   └── minimax.rs
│
├── channels/              # 渠道层 - 可插拔
│   ├── mod.rs           # Channel trait
│   ├── telegram/
│   ├── whatsapp/
│   └── qq/
│
├── tools/                 # 工具层
│   ├── mod.rs           # Tool trait
│   ├── web_search.rs    # Brave Search
│   └── fetch.rs         # Web Fetch
│
├── config/               # 配置层
│   └── mod.rs
│
├── types/                # 共享类型 - 解决循环依赖
│   └── mod.rs           # Message, LLMResponse, ToolCall 等
│
└── main.rs
```

### 模块职责

| 模块 | 职责 | 依赖 |
|------|------|------|
| `cli/` | 参数解析、命令路由 | `core/`, `config/` |
| `core/agent/` | Agent 逻辑执行 | `llm/`, `tools/`, `core/bus/` |
| `core/bus/` | 消息广播 | `types/` |
| `core/scheduler/` | 定时任务调度 | `core/bus/` |
| `llm/` | LLM 接口抽象 | `types/` |
| `channels/` | 各平台适配 | `core/bus/` |
| `tools/` | 外部工具封装 | `config/` |
| `types/` | 共享数据结构 | 无 |

---

## 三、关键技术改动

### 1. 提取共享类型 (解决循环依赖)

```rust
// src/types/mod.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub user_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
}
```

### 2. 解耦 CLI

```rust
// src/cli/commands/gateway.rs
use crate::config::Config;
use crate::core::{MessageBus, AgentExecutor, Scheduler, ChannelManager};

pub async fn execute(config: &Config) -> Result<()> {
    let bus = MessageBus::new();
    let agent = AgentExecutor::new(&config, &bus)?;
    let scheduler = Scheduler::new(&bus);
    let channels = ChannelManager::new(&bus, &config);

    // 并行启动所有组件
    tokio::try_join!(
        agent.run(),
        scheduler.run(),
        channels.run()
    )?;
    Ok(())
}
```

### 3. AgentExecutor

```rust
// src/core/agent/executor.rs
pub struct AgentExecutor {
    provider: Box<dyn LLMProvider>,
    tools: Vec<Box<dyn Tool>>,
    session: Session,
    bus: MessageBus,
}

impl AgentExecutor {
    pub async fn handle_message(&mut self, msg: &InboundMessage) -> Result<()> {
        // 添加用户消息到会话
        self.session.add_message(Message::user(&msg.content));

        // 工具调用循环
        let response = self.execute_with_tools().await?;

        // 添加助手回复到会话
        self.session.add_message(Message::assistant(&response.content));

        // 发布回复
        let outbound = OutboundMessage {
            channel: msg.channel.clone(),
            chat_id: msg.chat_id.clone(),
            content: response.content,
        };
        self.bus.publish_outbound(outbound).await;

        Ok(())
    }
}
```

---

## 四、分阶段开发计划

### Phase 1: 基础架构修复 (1-2周)

| 任务 | 描述 | 文件变动 |
|------|------|---------|
| 1.1 | 创建 `types/` 模块 | 新建 `src/types/mod.rs` |
| 1.2 | 提取共享类型 | 从 `agent.rs`, `providers.rs`, `bus.rs` 迁移 |
| 1.3 | 修复循环依赖 | 修改 `src/llm/mod.rs`, `src/core/agent/mod.rs` |
| 1.4 | 统一 `OutboundMessage` | 删除 `channels/mod.rs` 中的重复定义 |
| 1.5 | 清理未使用代码 | 删除 `skills.rs`, `memory.rs` 或实现集成 |

### Phase 2: 核心功能完善 (2-3周)

| 任务 | 描述 | 文件变动 |
|------|------|---------|
| 2.1 | 重构 `AgentExecutor` | 新建 `src/core/agent/executor.rs` |
| 2.2 | 支持消息历史累积 | 修改会话管理逻辑 |
| 2.3 | 集成 web 工具 | 修改 `AgentExecutor` 支持 `Tool` trait |
| 2.4 | 实现 CronExecutor 消息处理 | 新建 `src/core/scheduler/mod.rs` |
| 2.5 | 完善 Gateway 模式 | 修改 `src/cli/commands/gateway.rs` |

### Phase 3: 模块化解耦 (2-3周)

| 任务 | 描述 | 文件变动 |
|------|------|---------|
| 3.1 | 创建 `cli/commands/` 子模块 | 新建 `src/cli/commands/*.rs` |
| 3.2 | 提取 `llm/` 模块 | 从 `providers.rs` 重构 |
| 3.3 | 提取 `tools/` 模块 | 从 `agent/tools/mod.rs` 重构 |
| 3.4 | 提取 `core/scheduler/` | 从 `cron.rs` 重构 |
| 3.5 | 提取 `core/session/` | 从 `session.rs` 重构 |

### Phase 4: 工程化改进 (持续)

| 任务 | 描述 | 文件变动 |
|------|------|---------|
| 4.1 | 添加 `thiserror` 统一错误处理 | 新建 `src/errors.rs` |
| 4.2 | 添加 `Config::validate()` 配置验证 | 修改 `src/config/mod.rs` |
| 4.3 | 补充单元测试和集成测试 | 修改各模块添加 `#[cfg(test)]` |
| 4.4 | 添加日志分级和结构化日志 | 修改 `src/main.rs`, 各模块使用 `tracing` |
| 4.5 | 添加 Dockerfile 和 docker-compose.yml | 新建 `Dockerfile`, `docker-compose.yml` |

#### 4.4 日志分级实现

```rust
// src/main.rs
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("openat=info"));

    let subscriber = tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}
```

#### 4.5 Docker 支持

```dockerfile
# Dockerfile
FROM rust:1.93-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/openat /usr/local/bin/
ENTRYPOINT ["openat"]
```

```yaml
# docker-compose.yml
services:
  openat:
    build: .
    command: gateway
    volumes:
      - ~/.openat:/home/openat/.openat:rw
    environment:
      - RUST_LOG=openat=info
    restart: unless-stopped
    ports:
      - "18790:18790"
```

---

## 五、扩展指南

### 添加新 LLM 提供商

```rust
// src/llm/mod.rs
pub struct MyProvider {
    api_key: String,
    client: reqwest::Client,
}

#[async_trait::async_trait]
impl LLMProvider for MyProvider {
    async fn chat(&self, messages: &[Message]) -> Result<LLMResponse, String> {
        // 实现逻辑
    }
}
```

### 添加新渠道

```rust
// src/channels/my_channel.rs
pub struct MyChannel {
    bus: MessageBus,
    config: ChannelConfig,
}

#[async_trait::async_trait]
impl Channel for MyChannel {
    async fn run(&self) -> Result<()> {
        // 实现消息接收和发送
    }
}
```

### 添加新工具

```rust
// src/tools/my_tool.rs
pub struct MyTool;

#[async_trait::async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn definition(&self) -> Value {
        json!({...})
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        // 执行工具逻辑
    }
}
```

### 扩展点难度评级

| 扩展点 | 难度 |
|-------|------|
| 新渠道 | ⭐ |
| 新LLM提供商 | ⭐ |
| 新工具 | ⭐ |
| 新记忆系统 | ⭐⭐ |
| 数据库持久化 | ⭐⭐ |
| Web UI | ⭐⭐⭐ |

---

## 六、当前文件依赖关系

```
main.rs
    └── cli.rs
        ├── config/mod.rs
        ├── core/agent/mod.rs
        ├── core/scheduler/mod.rs
        └── core/channels/mod.rs

core/agent/mod.rs
    ├── llm/mod.rs (LLMProvider trait)
    ├── tools/mod.rs (Tool trait)
    ├── core/bus/mod.rs (MessageBus)
    └── core/session/mod.rs

llm/mod.rs
    └── types/mod.rs (LLMResponse, ToolCall)

channels/mod.rs
    └── core/bus/mod.rs (InboundMessage)

core/scheduler/mod.rs
    └── core/bus/mod.rs
```

---

## 七、参考命令

```bash
# 开发测试
cargo run --example test_search

# 代码检查
cargo clippy
cargo check

# 格式化
cargo fmt

# 运行测试
cargo test
```

---

## 八、后续优化方向

1. **数据库持久化**
   - 集成 SQLx 支持 PostgreSQL/SQLite
   - 会话历史持久化
   - 用户偏好存储

2. **监控和指标**
   - 添加 Prometheus 指标
   - 健康检查端点
   - 请求追踪

3. **安全加固**
   - API Key 加密存储
   - 输入验证和过滤
   - Rate limiting

4. **性能优化**
   - 连接池管理
   - 响应缓存
   - 并发控制

---

*文档生成时间: 2026-02-06*
*项目: nanobot - Ultra-Lightweight Personal AI Assistant*
