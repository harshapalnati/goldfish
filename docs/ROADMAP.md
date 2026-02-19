# Goldfish Integration Roadmap

To make Goldfish accessible to any application (Python, Node.js, HTTP), we will implement the following interfaces, similar to `mem0`.

## 1. HTTP REST API (Universal Access)
We will build a lightweight server using `axum` that exposes the Cortex as a microservice.

### Endpoints
*   `POST /v1/memories`: Remember something (with type/importance).
*   `GET /v1/context`: Get current working memory context.
*   `POST /v1/episodes/start`: Start a new episode.
*   `POST /v1/episodes/end`: End the current episode.
*   `GET /v1/search?q=...`: Search memories (Hybrid/Tantivy).

### Deployment
Run Goldfish as a sidecar container or standalone service:
```bash
./goldfish-server --port 8080 --data-dir ./memory
```

## 2. Python SDK (PyO3 Bindings)
For tight integration with Python AI frameworks (LangChain, LlamaIndex), we will wrap the Rust core using `PyO3`.

### Usage
```python
import goldfish

cortex = goldfish.MemoryCortex("./data")
cortex.remember("User likes AI", type="preference")
context = cortex.get_context()
```

## 3. Node.js SDK (Neon bindings)
Similar to Python, create native Node.js bindings for integration with TypeScript agents.

## Why this approach?
*   **Performance**: The core remains high-performance Rust.
*   **Flexibility**: HTTP API allows ANY language to connect.
*   **Ease of Use**: Native bindings provide idiomatic experience for Python/JS devs.

## 4. Model Context Protocol (MCP) Server
Implement Anthropic's **MCP** standard to allow Goldfish to be used as a "plug-and-play" memory server for any MCP-compliant LLM client (Claude Desktop, etc.).

### Features
*   Expose `read_resource` for fetching context.
*   Expose `call_tool` for remembering/searching.
*   Standardized JSON-RPC connection.
