# OpenAT 开发 TODO 列表

> 项目当前状态和待办事项

## 状态说明
- ✅ 已完成
- ⚠️ 部分完成 / 测试中
- ❌ 未实现

---

## 核心模块

| 功能 | 状态 | 说明 |
|------|------|------|
| Agent Executor | ✅ | 消息处理、工具调用循环 |
| MessageBus | ✅ | 消息队列、inbound/outbound |
| Config | ✅ | 配置加载和管理 |
| Gateway HTTP | ✅ | HTTP API 端点 |

---

## Tools 工具模块

| 功能 | 状态 | 说明 |
|------|------|------|
| web_search | ✅ | Web 搜索 (Brave Search) |
| web_fetch | ⚠️ | Web 内容获取 (需完善) |
| shell | ❌ | Shell 命令执行 |
| filesystem | ❌ | 文件系统操作 |

---

## Channels 渠道模块

| 功能 | 状态 | 说明 |
|------|------|------|
| Discord | ✅ | WebSocket Gateway + REST API |
| QQ | ✅ | 官方机器人 API (WebSocket + REST) |
| Telegram | ❌ | stub 实现，未完成 |
| WhatsApp | ❌ | 未实现 |

---

## Providers LLM 提供商

| 功能 | 状态 | 说明 |
|------|------|------|
| LLMProvider trait | ✅ | 统一接口 |
| OpenAI | ✅ | GPT-4o, GPT-4o-mini |
| Anthropic | ✅ | Claude 3.5 Sonnet |
| MiniMax | ✅ | MiniMax-M2.1 |
| OpenRouter | ✅ | 统一路由 |
| Groq | ✅ | 快速推理 |
| Gemini | ✅ | Google Gemini |
| DeepSeek | ❌ | 未完成 |
| 智谱 | ❌ | 未完成 |
| 月之暗面 | ❌ | 未完成 |

---

## Docker 部署

| 功能 | 状态 | 说明 |
|------|------|------|
| Dockerfile | ✅ | 多阶段构建 |
| docker-compose | ✅ | 部署配置 |
| Healthcheck | ✅ | 健康检查 |

---

## 待完成事项

### 高优先级

1. **完善 Telegram 渠道** - 实现完整的 Webhook/Long Polling
2. **完善 shell/filesystem 工具** - 安全风险需评估
3. **DeepSeek/智谱/月之暗面 Provider** - 添加更多 LLM 支持

### 中优先级

4. **WhatsApp 渠道** - 调研 Telegram MTProto 或 webhook 方案
5. **工具参数验证** - 增强安全性
6. **Rate Limiting** - 防止滥用

### 低优先级

7. **Web UI** - 管理界面
8. **数据库持久化** - 会话历史存储
9. **监控指标** - Prometheus 集成

---

*文档更新时间: 2026-02-22*
