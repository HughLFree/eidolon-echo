# HTTP API

Base URL: `http://127.0.0.1:3001`

OpenAPI: `GET /api/openapi.yaml`

## Health

- Method: `GET`
- Path: `/api/health`
- Response:

```json
{ "status": "ok" }
```

## Chat

- Method: `POST`
- Path: `/api/chat`
- Request:

```json
{
  "mode": "default",
  "message": "你好"
}
```

- Response:

```json
{
  "conversation_id": 1,
  "mode": "default",
  "reply": "你好！"
}
```

Notes:
- `mode` is optional: `default` or `roleplay` (`default` by default).
- `conversation_id` 由数据库决定并返回，客户端不应自行构造。
- `default` 模式只走内存缓存，不写 `messages` 表。
- `roleplay` 模式会写入 `messages` 表，并在冷启动/下拉翻页时从数据库补历史。
- Empty `message` returns `400`.

## Conversation Messages

- Method: `GET`
- Path: `/api/messages`
- Query: `limit` (optional, `1..200`, default `50`)
- Query: `mode` (optional, `default` or `roleplay`, default `default`)
- Query: `before_id` (optional, load older records where `id < before_id`)
- Order: newest first (latest message at top)
- Response:

```json
{
  "conversation_id": 1,
  "messages": [
    {
      "id": 1,
      "conversation_id": 1,
      "role": "user",
      "content": "你好",
      "created_at": "2026-03-04T10:00:00Z"
    }
  ]
}
```

## AI Providers CRUD

- `GET /api/ai-providers?with_disabled=true|false`
- `GET /api/ai-providers/{id}`
- `POST /api/ai-providers`
- `PUT /api/ai-providers/{id}`
- `DELETE /api/ai-providers/{id}`

Create request example:

```json
{
  "id": "openai-main",
  "name": "OpenAI Main",
  "provider_type": "openai_compat",
  "base_url": "https://api.openai.com/v1",
  "model_name": "gpt-4o-mini",
  "api_key_ref": "OPENAI_API_KEY",
  "enabled": true,
  "is_default": true,
  "temperature": 0.7,
  "max_tokens": 4096
}
```

## Profiles CRUD

- `GET /api/profiles?mode=default|roleplay`
- `GET /api/profiles/{id}`
- `POST /api/profiles`
- `PUT /api/profiles/{id}`
- `DELETE /api/profiles/{id}`

Create request example:

```json
{
  "id": "default-main",
  "mode": "default",
  "name": "默认助手",
  "avatar_path": "assets/avatars/default.png",
  "system_prompt": "你是一个简洁的桌面助手。",
  "opening_message": null,
  "context_limit": 12,
  "provider_id": "openai-main",
  "extra_json": null
}
```

Notes:
- `memory_enabled` is controlled by mode on server side and returned in response.
- `default -> memory_enabled=false`
- `roleplay -> memory_enabled=true`
