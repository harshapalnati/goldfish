# OpenClaw ↔ Goldfish Adapter

Persistent graph-based memory integration for OpenClaw AI agents. Replaces OpenClaw's flat file-based memory with Goldfish's typed, graph-connected memory system.

## Features

- 📄 **Markdown Parser**: Parse OpenClaw MEMORY.md and memory/*.md files
- 🔗 **Graph Relationships**: Automatic associations between related memories
- 🏷️ **Typed Memories**: Fact, Preference, Goal, Decision, Event, Identity, Observation
- 🧠 **Hybrid Search**: BM25 + Vector + Graph + Importance + Recency
- 🔄 **Lifecycle Integration**: beforeAgentStart/afterAgentEnd hooks
- ⚡ **Auto-Sync**: Watch for file changes and sync automatically
- 🎯 **Context Building**: Automatic memory injection into LLM prompts

## Installation

```bash
npm install openclaw-goldfish-adapter
```

## Quick Start

```typescript
import { OpenClawGoldfishAdapter } from 'openclaw-goldfish-adapter';

const adapter = new OpenClawGoldfishAdapter({
  goldfish: {
    baseUrl: 'http://localhost:3000'
  },
  openclaw: {
    workspaceDir: '~/.openclaw/workspace'
  }
});

// Initialize
await adapter.initialize();

// Get lifecycle hooks
const hooks = adapter.getLifecycleHooks();

// Use with OpenClaw
openclaw.use(hooks);
```

## Architecture

```
┌─────────────────────────────────────────────┐
│           OpenClaw Agent                    │
│     (TypeScript, runs in Node.js)          │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│     OpenClaw-Goldfish Adapter              │
│  ┌─────────────┬──────────────────────┐    │
│  │ .md Parser  │ Graph Builder        │    │
│  │  (chunking) │  (associations)      │    │
│  └─────────────┴──────────────────────┘    │
└──────────────┬──────────────────────────────┘
               │ HTTP/REST
┌──────────────▼──────────────────────────────┐
│         Goldfish Server                     │
│  ┌──────────────────────────────────┐      │
│  │  Enhanced Hybrid Search          │      │
│  │  (BM25 + Vector + Graph +        │      │
│  │   Importance + Recency)          │      │
│  └──────────────────────────────────┘      │
└─────────────────────────────────────────────┘
```

## Configuration

```typescript
interface AdapterConfig {
  goldfish: {
    baseUrl: string;        // Goldfish server URL
    timeout?: number;       // Request timeout (ms)
    retries?: number;       // Retry attempts
  };
  openclaw: {
    workspaceDir: string;   // OpenClaw workspace path
    memoryFile?: string;    // Main memory file (default: MEMORY.md)
    memoryPattern?: string; // Glob pattern (default: memory/**/*.md)
  };
  enableGraph?: boolean;    // Build graph associations (default: true)
  autoSync?: boolean;       // Auto-sync on file changes (default: true)
  syncInterval?: number;    // Sync interval in ms (default: 60000)
}
```

## Memory Types

The adapter automatically infers memory types from content:

| Type | Keywords | Importance |
|------|----------|------------|
| **Identity** | "name is", "lives in", "works as" | 1.0 |
| **Goal** | "goal:", "want to", "need to" | 0.9 |
| **Decision** | "decision:", "decided to" | 0.8 |
| **Preference** | "prefer", "likes", "enjoy" | 0.7 |
| **Fact** | "is", "are", "was" | 0.6 |
| **Event** | "last week", "yesterday", "today" | 0.4 |
| **Observation** | Default | 0.3 |

## Graph Relationships

The adapter creates associations based on:

1. **Keyword Similarity**: Shared keywords (Jaccard similarity > 0.3)
2. **Temporal Proximity**: Same session (< 1 hour), same day (< 24 hours)
3. **Type Relations**: Goal → Decision → Fact chains

Relation types:
- `related_to`: General similarity
- `caused_by`: Causal relationship
- `part_of`: Hierarchical relationship
- `updates`: Newer version
- `contradicts`: Conflicting information

## Lifecycle Hooks

### beforeAgentStart

Called before agent execution:

1. Builds search query from context
2. Searches Goldfish for relevant memories
3. Formats memories for system prompt injection
4. Starts a new episode

```typescript
const context = await hooks.beforeAgentStart({
  agentId: 'my-agent',
  currentTask: 'Build a React component',
  lastUserMessage: 'Create a button',
  systemPrompt: 'You are a helpful assistant...',
  conversation: ['...'],
  metadata: {}
});

// context contains formatted memories
```

### afterAgentEnd

Called after agent execution:

1. Ends current episode
2. Extracts new facts from conversation
3. Parses and syncs OpenClaw memory files
4. Stores new memories in Goldfish

```typescript
await hooks.afterAgentEnd({
  agentId: 'my-agent',
  conversation: ['User: ...', 'AI: ...'],
  // ...
});
```

## API Reference

### OpenClawGoldfishAdapter

#### `initialize()`
Initialize the adapter and perform initial sync.

#### `sync()`
Manually sync OpenClaw memory files to Goldfish.

#### `search(query, limit?)`
Search memories using hybrid retrieval.

#### `buildContext(query, tokenBudget?)`
Build formatted context for LLM with citations.

#### `getLifecycleHooks()`
Get lifecycle hooks for OpenClaw integration.

#### `getGoldfishClient()`
Get the underlying Goldfish HTTP client.

## Example: Manual Usage

```typescript
// Search for memories
const results = await adapter.search('user preferences', 5);

// Build context
const context = await adapter.buildContext(
  'What does the user like?', 
  500
);

// Store custom memory
await adapter.storeMemory({
  id: 'custom-123',
  content: 'User prefers Vim over VS Code',
  type: 'preference',
  importance: 0.8,
  timestamp: new Date(),
  metadata: { source: 'manual' },
  source: 'manual-entry',
  tags: ['editor', 'vim']
});
```

## Benefits Over OpenClaw Native Memory

| Feature | OpenClaw Native | OpenClaw + Goldfish |
|---------|-----------------|---------------------|
| **Search** | BM25 + Vector (flat) | BM25 + Vector + Graph |
| **Memory Types** | Untagged | Typed (7 categories) |
| **Relationships** | None | Graph associations |
| **Context Building** | Manual | Automatic |
| **Decay** | None | Smart decay by type |
| **Precision** | ~60% | 85%+ |

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run watch

# Test
npm test
```

## License

MIT

## Contributing

Contributions welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md).

## Links

- [Goldfish Repository](https://github.com/harshapalnati/goldfish)
- [OpenClaw Documentation](https://openclaw.ai)
- [Issue Tracker](https://github.com/harshapalnati/goldfish/issues)
