# Integrating Goldfish Memory with External Agents

Goldfish is designed as a standalone **Memory Cortex** that can act as the brain for *any* agent framework, including **OpenClaw**, **Rig**, or custom implementations.

## How to Integrate

### 1. Add Dependency
Add `goldfish` to your agent's `Cargo.toml`:

```toml
[dependencies]
goldfish = { git = "https://github.com/harshapalnati/goldfish", branch = "main" }
```

### 2. Initialize the Cortex
In your agent's initialization code:

```rust
use goldfish::{MemoryCortex, Memory, MemoryType};

struct MyAgent {
    // ... other fields
    memory: MemoryCortex,
}

impl MyAgent {
    pub async fn new() -> Self {
        let memory = MemoryCortex::new("./agent_memory_data").await.unwrap();
        Self { memory }
    }
}
```

### 3. Cycle of Cognition (The Loop)
To fully utilize Goldfish, your agent loop should follow this pattern:

```rust
async fn run_step(&self, user_input: &str) {
    // A. Start Episode (if new session)
    // self.memory.start_episode("User Interaction", "Chatting about X").await;

    // B. Recall Context
    // 1. "Pin" critical instructions (system prompt)
    // 2. Fetch relevant active memories
    let context = self.memory.get_context().await;
    
    // C. Act (LLM Call)
    let prompt = format!(
        "System: You are an AI.\nContext: {:?}\nUser: {}", 
        context, user_input
    );
    let responses = my_llm.generate(&prompt).await;

    // D. Learn (Store Memory)
    // Extract key facts/decisions from the interaction
    let memory = Memory::new(response, MemoryType::Decision);
    self.memory.remember(&memory).await.unwrap();
}
```

## Integration with Specific Frameworks

### OpenClaw
OpenClaw agents typically implement a `process` trait. You can inject `MemoryCortex` into the struct implementing this trait.

*   **State**: Store `MemoryCortex` in the agent state.
*   **Hooks**: Use `on_start` to load context and `on_completion` to save memories.

### Rig (Rust Intelligent Graph)
Rig agents use a pipeline specificiation. You can create a custom `MemoryTool` or `ContextProvider` implementation that wraps `goldfish::MemoryCortex`.

```rust
// Conceptual Rig integration
impl VectorStore for GoldfishWrapper {
    async fn search(&self, query: &str) -> Vec<Document> {
        self.cortex.recall(query, 5).await
            .into_iter().map(|m| Document::new(m.content)).collect()
    }
}
```

## Why it works for any agent
Goldfish is **agnostic** to the LLM or the control logic. It simply provides:
1.  **Input**: Text/Data to remember.
2.  **Output**: Structured Context to include in the prompt.

This means **ANY** system that can call Rust functions (or via FFI/WASM in the future) can use Goldfish as its long-term memory backend.
