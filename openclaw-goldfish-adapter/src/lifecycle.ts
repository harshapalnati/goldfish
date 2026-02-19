/**
 * OpenClaw lifecycle integration
 * Hooks for before/after agent execution
 */

import { 
  AgentContext, 
  ParsedMemory, 
  MemoryType,
  LifecycleHooks 
} from './types';
import { GoldfishClient } from './goldfish';
import { MarkdownParser } from './parser';
import { GraphBuilder } from './graph';

export class OpenClawLifecycle {
  private goldfish: GoldfishClient;
  private parser: MarkdownParser;
  private graphBuilder: GraphBuilder;
  private currentEpisodeId: string | null = null;

  constructor(
    goldfishClient: GoldfishClient,
    parser: MarkdownParser,
    graphBuilder: GraphBuilder
  ) {
    this.goldfish = goldfishClient;
    this.parser = parser;
    this.graphBuilder = graphBuilder;
  }

  /**
   * Called before agent starts - load relevant memories into context
   */
  async beforeAgentStart(context: AgentContext): Promise<string> {
    console.log(`[Goldfish] Loading memories for agent ${context.agentId}`);

    // Build query from context
    const query = this.buildQueryFromContext(context);
    
    // Search for relevant memories
    const memories = await this.goldfish.search(query, 10);
    
    // Format memories for system prompt
    const memoryContext = this.formatMemoriesForPrompt(memories);
    
    // Start a new episode
    try {
      const episode = await this.goldfish.startEpisode(
        context.currentTask || 'OpenClaw Session',
        context.metadata.sessionContext
      );
      this.currentEpisodeId = episode.id;
    } catch (error) {
      console.warn('[Goldfish] Failed to start episode:', error);
    }
    
    console.log(`[Goldfish] Loaded ${memories.length} memories into context`);
    
    return memoryContext;
  }

  /**
   * Called after agent ends - extract and store new memories
   */
  async afterAgentEnd(context: AgentContext): Promise<void> {
    console.log(`[Goldfish] Extracting memories from session ${context.agentId}`);

    // End current episode
    if (this.currentEpisodeId) {
      try {
        await this.goldfish.endEpisode(this.currentEpisodeId);
        this.currentEpisodeId = null;
      } catch (error) {
        console.warn('[Goldfish] Failed to end episode:', error);
      }
    }

    // Extract new facts from conversation
    const newMemories = this.extractNewMemories(context);
    
    // Store new memories
    if (newMemories.length > 0) {
      await this.goldfish.storeMemories(newMemories);
      console.log(`[Goldfish] Stored ${newMemories.length} new memories`);
    }

    // Parse and sync OpenClaw memory files
    await this.syncOpenClawFiles();
  }

  /**
   * Sync all OpenClaw memory files to Goldfish
   */
  async syncOpenClawFiles(): Promise<void> {
    console.log('[Goldfish] Syncing OpenClaw memory files...');

    try {
      // Parse all memories from files
      const memories = await this.parser.parseAllMemories();
      
      // Build graph associations
      const associations = this.graphBuilder.buildGraph(memories);
      
      // Sync to Goldfish
      const result = await this.goldfish.sync(memories, associations);
      
      console.log(`[Goldfish] Synced ${result.memoriesStored} memories, ${result.associationsCreated} associations`);
    } catch (error) {
      console.error('[Goldfish] Sync failed:', error);
    }
  }

  /**
   * Build search query from agent context
   */
  private buildQueryFromContext(context: AgentContext): string {
    const parts: string[] = [];
    
    if (context.currentTask) {
      parts.push(context.currentTask);
    }
    
    if (context.lastUserMessage) {
      parts.push(context.lastUserMessage);
    }
    
    // Extract keywords from conversation
    if (context.conversation.length > 0) {
      const lastMessages = context.conversation.slice(-3);
      parts.push(...lastMessages);
    }
    
    // Deduplicate and join
    const uniqueParts = [...new Set(parts)];
    return uniqueParts.join(' ').slice(0, 500); // Limit length
  }

  /**
   * Format memories for system prompt injection
   */
  private formatMemoriesForPrompt(memories: Array<{
    id: string;
    content: string;
    type: string;
    score: number;
  }>): string {
    if (memories.length === 0) {
      return '';
    }

    const lines: string[] = ['\n## Relevant Context from Memory\n'];
    
    for (let i = 0; i < memories.length; i++) {
      const mem = memories[i];
      lines.push(`${i + 1}. [${mem.type}] ${mem.content}`);
    }
    
    lines.push('\n');
    return lines.join('\n');
  }

  /**
   * Extract new memories from conversation
   */
  private extractNewMemories(context: AgentContext): ParsedMemory[] {
    const memories: ParsedMemory[] = [];
    const conversation = context.conversation.join('\n');
    
    // Pattern 1: User preferences
    const preferencePatterns = [
      /user (?:prefers?|likes?|enjoys?)\s+(.+?)(?:\.|\n|$)/gi,
      /user (?:dislikes?|hates?)\s+(.+?)(?:\.|\n|$)/gi
    ];
    
    for (const pattern of preferencePatterns) {
      let match;
      while ((match = pattern.exec(conversation)) !== null) {
        memories.push({
          id: this.generateId(),
          content: `User ${match[0]}`,
          type: 'preference',
          importance: 0.7,
          timestamp: new Date(),
          metadata: { source: 'conversation', agentId: context.agentId },
          source: 'openclaw-conversation',
          tags: ['preference', 'extracted']
        });
      }
    }
    
    // Pattern 2: Facts
    const factPatterns = [
      /user (?:is|are|was|were)\s+(.+?)(?:\.|\n|$)/gi,
      /user['']?s?\s+(?:name|age|location|job)\s+(?:is|=)\s+(.+?)(?:\.|\n|$)/gi
    ];
    
    for (const pattern of factPatterns) {
      let match;
      while ((match = pattern.exec(conversation)) !== null) {
        memories.push({
          id: this.generateId(),
          content: match[0],
          type: 'fact',
          importance: 0.6,
          timestamp: new Date(),
          metadata: { source: 'conversation', agentId: context.agentId },
          source: 'openclaw-conversation',
          tags: ['fact', 'extracted']
        });
      }
    }
    
    // Pattern 3: Decisions
    const decisionPatterns = [
      /(?:decided?|choose|opted?)\s+(?:to|for)\s+(.+?)(?:\.|\n|$)/gi
    ];
    
    for (const pattern of decisionPatterns) {
      let match;
      while ((match = pattern.exec(conversation)) !== null) {
        memories.push({
          id: this.generateId(),
          content: `Decision: ${match[1]}`,
          type: 'decision',
          importance: 0.8,
          timestamp: new Date(),
          metadata: { source: 'conversation', agentId: context.agentId },
          source: 'openclaw-conversation',
          tags: ['decision', 'extracted']
        });
      }
    }
    
    return memories;
  }

  /**
   * Generate a unique ID
   */
  private generateId(): string {
    return `openclaw_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Get lifecycle hooks for OpenClaw integration
   */
  getHooks(): LifecycleHooks {
    return {
      beforeAgentStart: this.beforeAgentStart.bind(this),
      afterAgentEnd: this.afterAgentEnd.bind(this)
    };
  }
}
