/**
 * Main adapter class that ties everything together
 */

import { 
  AdapterConfig, 
  LifecycleHooks,
  ParsedMemory,
  MemoryAssociation 
} from './types';
import { GoldfishClient } from './goldfish';
import { MarkdownParser } from './parser';
import { GraphBuilder } from './graph';
import { OpenClawLifecycle } from './lifecycle';

export class OpenClawGoldfishAdapter {
  private config: AdapterConfig;
  private goldfish: GoldfishClient;
  private parser: MarkdownParser;
  private graphBuilder: GraphBuilder;
  private lifecycle: OpenClawLifecycle;
  private isInitialized: boolean = false;

  constructor(config: AdapterConfig) {
    this.config = {
      enableGraph: true,
      autoSync: true,
      syncInterval: 60000, // 1 minute
      ...config
    };

    // Initialize components
    this.goldfish = new GoldfishClient(config.goldfish);
    this.parser = new MarkdownParser(
      config.openclaw.workspaceDir,
      config.openclaw.memoryFile,
      config.openclaw.memoryPattern
    );
    this.graphBuilder = new GraphBuilder();
    this.lifecycle = new OpenClawLifecycle(
      this.goldfish,
      this.parser,
      this.graphBuilder
    );
  }

  /**
   * Initialize the adapter
   * - Check Goldfish connection
   * - Sync existing memories
   * - Set up file watching if autoSync is enabled
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    console.log('[OpenClaw-Goldfish] Initializing adapter...');

    try {
      // Check Goldfish health
      const health = await this.goldfish.health();
      console.log(`[OpenClaw-Goldfish] Connected to Goldfish v${health.version}`);

      // Initial sync
      if (this.config.autoSync) {
        await this.sync();
      }

      // Set up file watching
      if (this.config.autoSync) {
        this.setupFileWatching();
      }

      this.isInitialized = true;
      console.log('[OpenClaw-Goldfish] Adapter initialized successfully');
    } catch (error) {
      console.error('[OpenClaw-Goldfish] Initialization failed:', error);
      throw error;
    }
  }

  /**
   * Sync OpenClaw memory files to Goldfish
   */
  async sync(): Promise<{ memories: number; associations: number }> {
    console.log('[OpenClaw-Goldfish] Syncing memories...');

    try {
      // Parse all memories
      const memories = await this.parser.parseAllMemories();
      
      // Build graph associations
      let associations: MemoryAssociation[] = [];
      if (this.config.enableGraph) {
        associations = this.graphBuilder.buildGraph(memories);
      }

      // Sync to Goldfish
      const result = await this.goldfish.sync(memories, associations);

      console.log(`[OpenClaw-Goldfish] Synced ${result.memoriesStored} memories, ${result.associationsCreated} associations`);
      
      return {
        memories: result.memoriesStored,
        associations: result.associationsCreated
      };
    } catch (error) {
      console.error('[OpenClaw-Goldfish] Sync failed:', error);
      throw error;
    }
  }

  /**
   * Search memories
   */
  async search(query: string, limit?: number) {
    return this.goldfish.search(query, limit);
  }

  /**
   * Build context for LLM
   */
  async buildContext(query: string, tokenBudget?: number) {
    return this.goldfish.buildContext(query, tokenBudget);
  }

  /**
   * Store a single memory
   */
  async storeMemory(memory: ParsedMemory) {
    return this.goldfish.storeMemory(memory);
  }

  /**
   * Get lifecycle hooks for OpenClaw integration
   */
  getLifecycleHooks(): LifecycleHooks {
    return this.lifecycle.getHooks();
  }

  /**
   * Get the Goldfish client for advanced usage
   */
  getGoldfishClient(): GoldfishClient {
    return this.goldfish;
  }

  /**
   * Get the Markdown parser
   */
  getParser(): MarkdownParser {
    return this.parser;
  }

  /**
   * Get the graph builder
   */
  getGraphBuilder(): GraphBuilder {
    return this.graphBuilder;
  }

  /**
   * Manually trigger memory extraction from conversation
   */
  async extractMemories(conversation: string[], context?: Record<string, any>) {
    // This would be called from the afterAgentEnd hook
    // But can also be called manually
    await this.lifecycle.afterAgentEnd({
      agentId: context?.agentId || 'manual',
      conversation,
      systemPrompt: '',
      metadata: context || {}
    });
  }

  /**
   * Setup file watching for auto-sync
   */
  private setupFileWatching(): void {
    this.parser.watchForChanges(async (filePath) => {
      console.log(`[OpenClaw-Goldfish] Memory file changed: ${filePath}`);
      
      // Debounce - wait a bit for writes to complete
      setTimeout(async () => {
        try {
          await this.sync();
        } catch (error) {
          console.error('[OpenClaw-Goldfish] Auto-sync failed:', error);
        }
      }, 1000);
    });

    console.log('[OpenClaw-Goldfish] File watching enabled');
  }

  /**
   * Dispose of resources
   */
  dispose(): void {
    console.log('[OpenClaw-Goldfish] Disposing adapter...');
    this.isInitialized = false;
  }
}
