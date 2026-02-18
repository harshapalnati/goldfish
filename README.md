<div align="center">

![Goldfish Banner](https://raw.githubusercontent.com/harshapalnati/goldfish/main/assets/banner.png)

# Goldfish

### Give Your AI Agent a Memory

**🧠 Smart Recall** • **🔍 Hybrid Search** • **🎯 Context Aware**

[![API](https://img.shields.io/badge/API-REST-blue)](http://localhost:3000)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.0-green)](openapi.yaml)
[![Python](https://img.shields.io/badge/Python-3.8+-blue)](examples/goldfish_client.py)
[![JavaScript](https://img.shields.io/badge/JavaScript-ES6+-yellow)](examples/js_client.js)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%2FMIT-blue.svg)](LICENSE)

[🚀 Quick Start](#quick-start) • [📖 API Docs](openapi.yaml) • [💻 Examples](#quick-integration) • [🌟 Star History](#star-history)

</div>

---

## 🚀 Quick Start (30 Seconds)

### 1. Start the Server

```bash
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish
cargo run --example server --features dashboard
```

✅ **Server running on http://localhost:3000**

### 2. Make Your First API Call

```bash
# Store a memory
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

### 3. Search with Hybrid Ranking

```bash
curl -X POST http://localhost:3000/v1/search \
  -H "Content-Type: application/json" \
  -d '{"query": "user preferences", "limit": 5}'
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
  "citations": [{"id": "mem_abc123", "content": "...", "type": "Preference"}]
}
```

---

## 💻 Quick Integration

### Python

```python
from goldfish_client import GoldfishClient

client = GoldfishClient("http://localhost:3000")

# Store memory
client.remember("User prefers dark mode", type="preference", importance=0.9)

# Search
results = client.recall("user preferences", limit=5)

# Build LLM context
context = client.context("What does user prefer?", token_budget=500)
print(context["context"])  # Ready for your LLM prompt!
```

### JavaScript

```javascript
import GoldfishClient from './js_client.js';

const client = new GoldfishClient('http://localhost:3000');

// Store memory
await client.remember('User prefers dark mode', 'preference', 0.9);

// Search
const results = await client.recall('user preferences', 5);

// Build context
const ctx = await client.context('What does user prefer?', 500);
console.log(ctx.context); // Ready for your LLM!
```

### Rust

```rust
use goldfish::{MemoryCortex, Memory, MemoryType};

let cortex = MemoryCortex::new("./data").await?;

// Store
cortex.remember(&Memory::new(
    "User prefers dark mode",
    MemoryType::Preference
)).await?;

// Search
let results = cortex.recall("user preferences", 5).await?;

// Context
let context = cortex.get_full_context(10).await?;
```

---

## 📖 API Reference

**[📘 OpenAPI Specification →](openapi.yaml)**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/memory` | Store a memory |
| `GET` | `/v1/memory/:id` | Get memory by ID |
| `POST` | `/v1/search` | Hybrid search (BM25 + Vector + Importance + Recency) |
| `POST` | `/v1/context` | Build LLM context with citations |
| `POST` | `/v1/episodes/start` | Start episodic experience |
| `POST` | `/v1/episodes/:id/end` | End episode |
| `GET` | `/health` | Health check |

---

## ✨ Why Goldfish?

| Feature | Benefit |
|---------|---------|
| **🌐 Works Everywhere** | Python, JavaScript, Rust, Go, or any language |
| **🔧 Framework Agnostic** | LangChain, LlamaIndex, CrewAI, or custom agents |
| **⚡ Zero Lock-in** | Simple HTTP calls, no heavy dependencies |
| **🔒 Secure** | Run locally or deploy to your infrastructure |
| **📈 Production Ready** | SQLite locally → PostgreSQL in production |

---

## 🧠 Features

### Hybrid Search
```
Score = BM25×0.35 + Vector×0.35 + Recency×0.20 + Importance×0.10
```

- **BM25**: Full-text search
- **Vector**: Semantic similarity (cosine)
- **Recency**: Time decay boost
- **Importance**: Type-based scoring

### Working Memory
- Fast LRU cache (20 items default)
- Pin critical memories
- Automatic attention scoring

### Episodic Memory
- Group memories into experiences
- Start/end episode API
- Temporal context tracking

### Context Builder
- Token-budgeted generation
- Citations with memory IDs
- Explainability ("why included")

---

## 🏗️ Architecture

```
┌─────────────────────────────────────┐
│           Your Agent                │
│    (Python/JS/Rust/Go/Any)         │
└─────────────┬───────────────────────┘
              │ HTTP/JSON
┌─────────────▼───────────────────────┐
│      Goldfish Memory API            │
│  ┌──────────┬──────────┐           │
│  │  BM25    │  Vector  │  Hybrid   │
│  │ (Tantivy)│ (Cosine) │  Search   │
│  └──────────┴──────────┘           │
│  ┌─────────────────────────────┐   │
│  │  Working Memory (LRU)       │   │
│  │  Episodes                   │   │
│  │  Context Builder            │   │
│  └─────────────────────────────┘   │
└──────────────────┬──────────────────┘
                   │
┌──────────────────▼──────────────────┐
│        Storage Backend              │
│  ┌─────────────────────────────┐   │
│  │  SQLite (default)           │   │
│  │  PostgreSQL (production)    │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

---

## 🚀 Installation

### Docker (Coming Soon)
```bash
docker run -p 3000:3000 goldfish/memory:latest
```

### From Source
```bash
git clone https://github.com/harshapalnati/goldfish.git
cd goldfish
cargo build --release
./target/release/goldfish-server
```

---

## 🌟 Star History

[![Star History Chart](https://api.star-history.com/svg?repos=harshapalnati/goldfish&type=Date)](https://star-history.com/#harshapalnati/goldfish&Date)

---

## 📊 Comparison

| Feature | Spacebot | Goldfish |
|---------|----------|----------|
| **API** | ❌ No | ✅ REST API |
| **Hybrid Search** | ❌ Text only | ✅ BM25 + Vector |
| **Working Memory** | ❌ No | ✅ LRU Cache |
| **Episodes** | ❌ No | ✅ Grouped experiences |
| **Context Builder** | ❌ Manual | ✅ Automatic with citations |
| **Multi-language** | ❌ No | ✅ Any language via HTTP |
| **Explainability** | ❌ No | ✅ "Why retrieved" |

---

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Contact:** harshapalnati@gmail.com

---

## 📄 License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).

---

<div align="center">

**Made with 🐠 by [Harsha Palnati](https://github.com/harshapalnati)**

⭐ Star us if Goldfish helps your agents remember!

</div>
