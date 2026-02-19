/**
 * OpenClaw Goldfish Adapter
 * 
 * Integrates Goldfish persistent memory with OpenClaw AI agents
 * Features:
 * - Parse OpenClaw Markdown memory files
 * - Build graph relationships between memories
 * - Automatic memory extraction from conversations
 * - Typed memory storage (Fact, Preference, Goal, Decision, Event)
 * - Hybrid search (BM25 + Vector + Graph)
 * 
 * Usage:
 * ```typescript
 * import { OpenClawGoldfishAdapter } from 'openclaw-goldfish-adapter';
 * 
 * const adapter = new OpenClawGoldfishAdapter({
 *   goldfish: { baseUrl: 'http://localhost:3000' },
 *   openclaw: { workspaceDir: '~/.openclaw/workspace' }
 * });
 * 
 * // Initialize
 * await adapter.initialize();
 * 
 * // Get lifecycle hooks
 * const hooks = adapter.getLifecycleHooks();
 * 
 * // Use with OpenClaw
 * openclaw.use(hooks);
 * ```
 */

export { OpenClawGoldfishAdapter } from './adapter';
export { GoldfishClient } from './goldfish';
export { MarkdownParser } from './parser';
export { GraphBuilder } from './graph';
export { OpenClawLifecycle } from './lifecycle';

// Export types
export * from './types';
