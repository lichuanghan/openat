# OpenAT 测试计划

> 测试版本: v1.0.0
> 更新日期: 2026-02-22

---

## 已测试功能

### 渠道测试

| 渠道 | 状态 | 说明 |
|------|------|------|
| Discord | ✅ | WebSocket Gateway 连接成功，消息收发正常 |
| QQ | ✅ | 官方机器人 API 连接成功，消息收发正常 |

### Provider 测试

| Provider | 状态 | 模型 |
|---------|------|------|
| MiniMax | ✅ | M2.1 |
| OpenRouter | ✅ | - |

### HTTP API 测试

| 端点 | 状态 |
|------|------|
| GET /health | ✅ |
| GET / | ✅ |
| POST /chat | ✅ |

### Docker 部署

| 测试项 | 状态 |
|--------|------|
| docker build | ✅ |
| docker-compose up | ✅ |
| Healthcheck | ✅ |

---

## 待测试功能

- Telegram 渠道完整实现
- WhatsApp 渠道
- Shell/Filesystem 工具 (安全考虑暂未实现)
- 其他 LLM Provider

---

## 测试命令

```bash
# 本地运行测试
cargo run -p openat-cli -- gateway

# Docker 测试
docker-compose up -d
docker-compose logs -f
```

---

*更新日期: 2026-02-22*
