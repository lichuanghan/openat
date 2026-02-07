# Openat 开发 TODO 列表

> 对比 Python nanobot 版本，列出缺失功能并逐步实现

## 状态说明
- ✅ 已完成
- ⚠️ 部分完成
- ❌ 未实现

---

## Agent 模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| loop/executor | ✅ | P0 | Agent 执行循环 |
| memory | ✅ | P0 | 记忆管理 |
| skills | ✅ | P0 | 技能系统 |
| context | ✅ | P1 | 上下文管理 |
| subagent | ✅ | P2 | 子代理功能 |

---

## Tools 工具模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| registry | ✅ | P0 | 工具注册表 |
| web_search | ✅ | P0 | Web 搜索 (Brave) |
| web_fetch | ✅ | P0 | Web 内容获取 (增强) |
| shell | ✅ | P1 | Shell 命令执行 |
| filesystem | ✅ | P1 | 文件系统操作 |
| cron | ✅ | P1 | 定时任务工具 |
| spawn | ✅ | P2 | 进程启动工具 |
| message | ✅ | P2 | 消息发送工具 |

---

## Bus 消息总线

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| queue | ✅ | P0 | 消息队列 |
| events | ✅ | P1 | 事件系统 |

---

## Channels 渠道模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| telegram | ✅ | P0 | Telegram |
| whatsapp | ✅ | P0 | WhatsApp |
| qq | ✅ | P0 | QQ |
| manager | ✅ | P1 | 渠道管理器 (基础) |
| feishu | ✅ | P2 | 飞书 (Lark) |
| discord | ✅ | P2 | Discord |

---

## Providers LLM 提供商

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| base | ✅ | P0 | Provider 基础接口 |
| openrouter | ✅ | P0 | OpenRouter |
| openai | ✅ | P0 | OpenAI |
| anthropic | ✅ | P0 | Anthropic |
| groq | ✅ | P0 | Groq |
| gemini | ✅ | P0 | Gemini |
| minimax | ✅ | P0 | MiniMax |
| litellm | ✅ | P2 | LiteLLM 统一接口 |
| transcription | ✅ | P3 | 语音转录 (Groq Whisper API) |
| deepseek | ✅ | P3 | DeepSeek |
| zhipu | ✅ | P3 | 智谱 (ChatGLM) |
| moonshot | ✅ | P3 | 月之暗面 (Kimi) |
| vllm | ✅ | P3 | VLLM 本地部署 |

---

## Skills 技能模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| cron | ✅ | P0 | 定时任务技能 |
| github | ✅ | P2 | GitHub 操作 (gh CLI) |
| weather | ✅ | P3 | 天气查询 (wttr.in) |
| summarize | ✅ | P3 | 文本总结 (summarize.sh) |
| tmux | ✅ | P3 | Tmux 集成 |

---

## Session 模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| manager | ✅ | P0 | 会话管理 |

---

## Cron 模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| scheduler | ✅ | P0 | 调度器 |
| service | ✅ | P0 | 定时服务 |

---

## Heartbeat 模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| heartbeat | ✅ | P0 | 心跳监控 |

---

## 实现顺序 (所有 Phase 已完成)

### Phase 1: 工具补全 ✅
- ✅ shell, filesystem, cron_tool, spawn, message

### Phase 2: 上下文和事件 ✅
- ✅ context, events

### Phase 3: 子代理 ✅
- ✅ subagent

### Phase 4: 扩展提供商 ✅
- ✅ litellm, deepseek, zhipu, moonshot, vllm, transcription

### Phase 5: 新渠道 ✅
- ✅ feishu, discord

### Phase 6: 技能扩展 ✅
- ✅ github, weather, summarize, tmux

---

## 进度追踪

**所有功能已完成实现！** Rust 版本的 openat 现在与 Python nanobot 功能对齐。

### 已完成 ✅
- ✅ Agent 模块 (loop, memory, skills, context, subagent)
- ✅ Tools (shell, filesystem, cron, spawn, message, web_search, web_fetch)
- ✅ Providers (OpenAI, Anthropic, Groq, Gemini, MiniMax, DeepSeek, Zhipu, Moonshot, VLLM, LiteLLM, Transcription)
- ✅ Channels (Telegram, WhatsApp, QQ, Discord, Feishu)
- ✅ Bus (消息队列, 事件系统)
- ✅ Skills (cron, github, weather, summarize, tmux)
- ✅ Session, Cron, Heartbeat

### 下一步优化方向
- WebSocket 实时消息支持
- 工具参数验证增强
- **安全修复**: 命令注入防护、目录遍历限制

---

**测试状态**: 73/110 (68%) - 57 单元测试通过, 性能测试通过, 发现 3 个安全漏洞

*文档更新时间: 2026-02-07*
