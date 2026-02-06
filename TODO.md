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
| context | ❌ | P1 | 上下文管理 |
| subagent | ❌ | P2 | 子代理功能 |

---

## Tools 工具模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| registry | ✅ | P0 | 工具注册表 |
| web_search | ⚠️ | P0 | Web 搜索 (Brave) |
| web_fetch | ⚠️ | P0 | Web 内容获取 |
| shell | ❌ | P1 | Shell 命令执行 |
| filesystem | ❌ | P1 | 文件系统操作 |
| cron | ❌ | P1 | 定时任务工具 |
| spawn | ❌ | P2 | 进程启动工具 |

---

## Bus 消息总线

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| queue | ✅ | P0 | 消息队列 |
| events | ❌ | P1 | 事件系统 |

---

## Channels 渠道模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| telegram | ✅ | P0 | Telegram |
| whatsapp | ✅ | P0 | WhatsApp |
| manager | ❌ | P1 | 渠道管理器 |
| feishu | ❌ | P2 | 飞书 |
| discord | ❌ | P2 | Discord |

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
| litellm | ❌ | P2 | LiteLLM 统一接口 |
| transcription | ❌ | P3 | 语音转录 |

---

## Skills 技能模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| cron | ⚠️ | P0 | 定时任务技能 |
| github | ❌ | P2 | GitHub 操作 |
| weather | ❌ | P3 | 天气查询 |
| summarize | ❌ | P3 | 文本总结 |
| tmux | ❌ | P3 | Tmux 集成 |
| skill-creator | ❌ | P3 | 技能创建器 |

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
| service | ⚠️ | P0 | 定时服务 |

---

## Heartbeat 模块

| 功能 | 状态 | 优先级 | 说明 |
|------|------|--------|------|
| heartbeat | ✅ | P0 | 心跳监控 |

---

## 实现顺序

### Phase 1: 工具补全 (P1)

1. **shell** - Shell 命令执行工具
2. **filesystem** - 文件系统操作工具
3. **cron_tool** - 定时任务工具
4. **spawn** - 进程启动工具

### Phase 2: 上下文和事件 (P1)

5. **context** - 上下文管理
6. **events** - 事件系统

### Phase 3: 子代理 (P2)

7. **subagent** - 子代理功能

### Phase 4: 扩展提供商 (P2)

8. **litellm** - LiteLLM 统一接口

### Phase 5: 新渠道 (P2)

9. **feishu** - 飞书渠道
10. **discord** - Discord 渠道
11. **manager** - 渠道管理器

### Phase 6: 技能扩展 (P2-P3)

12. **github** - GitHub 操作技能
13. **weather** - 天气查询技能
14. **summarize** - 总结技能
15. **tmux** - Tmux 集成技能

---

## 进度追踪

- [ ] Phase 1: 工具补全
  - [ ] shell
  - [ ] filesystem
  - [ ] cron_tool
  - [ ] spawn
- [ ] Phase 2: 上下文和事件
  - [ ] context
  - [ ] events
- [ ] Phase 3: 子代理
  - [ ] subagent
- [ ] Phase 4: 扩展提供商
  - [ ] litellm
- [ ] Phase 5: 新渠道
  - [ ] feishu
  - [ ] discord
  - [ ] manager
- [ ] Phase 6: 技能扩展
  - [ ] github
  - [ ] weather
  - [ ] summarize
  - [ ] tmux

---

*文档更新时间: 2026-02-06*
