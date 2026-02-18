# 🚀 Quick Start Guide

Get Goldfish running in 5 minutes with a simple HTTP API.

## Option 1: Use the Pre-built Server (Easiest)

### 1. Start the Server

```bash
# Clone and run
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish
cargo run --example server --features dashboard
```

Server starts on `http://localhost:3000` 🎉

### 2. Store a Memory (curl)

```bash
curl -X POST http://localhost:3000/v1/memory \
  -H "Content-Type: application/json" \
  -d '{
    "content": "User prefers dark mode in all applications",
    "type": "preference",
    "importance": 0.9
  }'
```

**Response:**
```json
{
  "id": "mem_abc123",
  "content": "User prefers dark mode in all applications",
  "type": "Preference",
  "importance": 0.9,
  "created_at": "2026-02-18T10:30:00Z"
}
```

### 3. Search Memories

```bash
curl -X POST http://localhost:3000/v1/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "user preferences",
    "limit": 5
  }'
```

**Response:**
```json
[
  {
    "id": "mem_abc123",
    "content": "User prefers dark mode in all applications",
    "type": "Preference",
    "score": 0.95,
    "why": "Matched query 'user preferences' with score 0.95"
  }
]
```

### 4. Build LLM Context

```bash
curl -X POST http://localhost:3000/v1/context \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What does the user prefer?",
    "token_budget": 500
  }'
```

**Response:**
```json
{
  "context": "## Relevant Context\n1 [Preference] User prefers dark mode in all applications\n",
  "tokens_used": 12,
  "memories_included": 1,
  "citations": [
    {
      "id": "mem_abc123",
      "content": "User prefers dark mode in all applications",
      "type": "Preference"
    }
  ]
}
```

## Option 2: Python Client

```python
# Install: pip install requests
import requests

class GoldfishClient:
    def __init__(self, url="http://localhost:3000"):
        self.url = url
    
    def remember(self, content, type="fact", importance=None):
        data = {"content": content, "type": type}
        if importance:
            data["importance"] = importance
        r = requests.post(f"{self.url}/v1/memory", json=data)
        return r.json()
    
    def recall(self, query, limit=10):
        r = requests.post(f"{self.url}/v1/search", 
                         json={"query": query, "limit": limit})
        return r.json()
    
    def context(self, query, token_budget=2000):
        r = requests.post(f"{self.url}/v1/context",
                         json={"query": query, "token_budget": token_budget})
        return r.json()

# Usage
client = GoldfishClient()

# Store
client.remember("User likes Python", "preference", importance=0.9)

# Search
results = client.recall("python")

# Build context for LLM
ctx = client.context("What programming language?", token_budget=500)
print(ctx["context"])  # Ready to use in prompt!
```

## Option 3: Rust Integration

```rust
use goldfish::{MemoryCortex, Memory, MemoryType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize
    let cortex = MemoryCortex::new("./data").await?;
    
    // Store
    cortex.remember(&Memory::new(
        "User prefers dark mode",
        MemoryType::Preference
    )).await?;
    
    // Search
    let results = cortex.recall("preferences", 10).await?;
    
    // Build context
    let context = cortex.get_full_context(10).await?;
    
    Ok(())
}
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/memory` | Store a memory |
| GET | `/v1/memory/:id` | Get memory by ID |
| POST | `/v1/search` | Search memories |
| POST | `/v1/context` | Build LLM context |
| POST | `/v1/episodes/start` | Start an episode |
| POST | `/v1/episodes/:id/end` | End an episode |
| GET | `/health` | Health check |

## Memory Types

- **`fact`** - Objective information
- **`preference`** - User preferences (high importance)
- **`goal`** - Objectives to achieve
- **`decision`** - Choices made
- **`event`** - Something that happened
- **`identity`** - Core characteristics

## Features Included

✅ **Hybrid Retrieval** - BM25 + recency + importance + graph  
✅ **Context Builder** - Token-budgeted with citations  
✅ **Episodes** - Group memories into experiences  
✅ **Explanability** - Know why each memory was retrieved  
✅ **Working Memory** - Fast cache for active context  

## Next Steps

1. **Vector Search** - Enable `lancedb` feature for semantic search
2. **Python SDK** - `pip install goldfish` (coming soon)
3. **TypeScript SDK** - `npm install @goldfish/sdk` (coming soon)

## Docker (Coming Soon)

```bash
docker run -p 3000:3000 goldfish/memory:latest
```

---

**Questions?** Open an issue on GitHub!
