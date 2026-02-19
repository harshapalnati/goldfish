# Goldfish Server

A lightweight, standalone memory server for AI agents. It wraps the powerful Goldfish Memory Cortex in a simple REST API.

## 🚀 Quick Start

### 1. Build & Run
```bash
cargo run -p goldfish-server
```
The server will start on `http://localhost:3000`. data will be stored in `./goldfish_data`.

### 2. Connect Your Agent
Use the provided Python client example or any HTTP client.

```python
# Copy examples/client.py to your project
from client import remember, get_context

# 1. Store a memory
remember("The user is working on a Rust project", "fact")

# 2. Get Agent Context (What to put in System Prompt)
context = get_context()
print(context['formatted_context'])
```

## API Endpoints

| Method | Endpoint | Description | Payload |
|:---|:---|:---|:---|
| `POST` | `/v1/memory` | Store a new memory | `{ "content": "...", "type": "fact", "importance": 0.5 }` |
| `GET` | `/v1/search` | Search memories | `?q=query&limit=5` |
| `GET` | `/v1/context` | Get working memory & context | - |

## Integration

### Python / LangChain
See `examples/client.py` for a full wrapper.

### Curl
```bash
curl -X POST http://localhost:3000/v1/memory \
  -H "Content-Type: application/json" \
  -d '{"content": "I like pizza", "memory_type": "preference"}'
```
