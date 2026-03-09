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
  "session_id": 1,
  "mode": "default",
  "message": "你好"
}
```

- Response:

```json
{
  "session_id": 1,
  "mode": "default",
  "reply": "你好！"
}
```

Notes:
- `mode` is optional: `default` or `roleplay` (`default` by default).
- If `session_id` is `null` or omitted, backend creates an in-memory session id.
- Empty `message` returns `400`.

## Session Messages

- Method: `GET`
- Path: `/api/sessions/{session_id}/messages`
- Query: `limit` (optional, `1..200`, default `50`)
- Query: `mode` (optional, `default` or `roleplay`, default `default`)
- Query: `before_id` (optional, load older records where `id < before_id`)
- Order: newest first (latest message at top)
- Response:

```json
{
  "session_id": 1,
  "messages": [
    {
      "id": 1,
      "session_id": 1,
      "role": "user",
      "content": "你好",
      "created_at": "2026-03-04T10:00:00Z"
    }
  ]
}
```
