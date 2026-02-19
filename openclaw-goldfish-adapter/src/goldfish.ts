/**
 * HTTP client for Goldfish Memory API
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import { 
  GoldfishConfig, 
  ParsedMemory, 
  MemoryAssociation, 
  SearchResult,
  MemoryType 
} from './types';

export class GoldfishClient {
  private client: AxiosInstance;
  private config: GoldfishConfig;

  constructor(config: GoldfishConfig) {
    this.config = {
      timeout: 5000,
      retries: 3,
      ...config
    };

    this.client = axios.create({
      baseURL: this.config.baseUrl,
      timeout: this.config.timeout,
      headers: {
        'Content-Type': 'application/json'
      }
    });

    // Add retry logic
    this.setupRetryLogic();
  }

  private setupRetryLogic(): void {
    this.client.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        const config = error.config;
        if (!config) return Promise.reject(error);

        // @ts-ignore - add retry count to config
        config.retryCount = config.retryCount || 0;

        // @ts-ignore
        if (config.retryCount < (this.config.retries || 3)) {
          // @ts-ignore
          config.retryCount++;
          // Wait before retrying (exponential backoff)
          const delay = Math.pow(2, config.retryCount) * 100;
          await new Promise(resolve => setTimeout(resolve, delay));
          return this.client(config);
        }

        return Promise.reject(error);
      }
    );
  }

  /**
   * Health check
   */
  async health(): Promise<{ status: string; version: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }

  /**
   * Store a memory in Goldfish
   */
  async storeMemory(memory: ParsedMemory): Promise<{ id: string }> {
    const response = await this.client.post('/v1/memory', {
      content: memory.content,
      type: memory.type,
      importance: memory.importance,
      source: memory.source,
      metadata: memory.metadata
    });
    return response.data;
  }

  /**
   * Store multiple memories in batch
   */
  async storeMemories(memories: ParsedMemory[]): Promise<void> {
    // Store sequentially to avoid overwhelming the server
    for (const memory of memories) {
      try {
        await this.storeMemory(memory);
      } catch (error) {
        console.error(`Failed to store memory ${memory.id}:`, error);
        // Continue with other memories
      }
    }
  }

  /**
   * Get a memory by ID
   */
  async getMemory(id: string): Promise<SearchResult | null> {
    try {
      const response = await this.client.get(`/v1/memory/${id}`);
      return response.data;
    } catch (error) {
      if ((error as AxiosError).response?.status === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Search memories
   */
  async search(query: string, limit: number = 10): Promise<SearchResult[]> {
    const response = await this.client.post('/v1/search', {
      query,
      limit
    });
    return response.data;
  }

  /**
   * Build context for LLM
   */
  async buildContext(query: string, tokenBudget: number = 2000): Promise<{
    context: string;
    tokensUsed: number;
    memoriesIncluded: number;
    citations: Array<{
      id: string;
      content: string;
      type: MemoryType;
    }>;
  }> {
    const response = await this.client.post('/v1/context', {
      query,
      token_budget: tokenBudget
    });
    return response.data;
  }

  /**
   * Create an association between memories
   */
  async createAssociation(association: MemoryAssociation): Promise<void> {
    // Note: This assumes Goldfish has an associations endpoint
    // If not, you may need to store associations differently
    try {
      await this.client.post('/v1/associate', {
        source_id: association.sourceId,
        target_id: association.targetId,
        relation: association.relation,
        weight: association.weight
      });
    } catch (error) {
      // Association endpoint might not exist yet
      console.warn('Association creation not supported or failed:', error);
    }
  }

  /**
   * Create multiple associations
   */
  async createAssociations(associations: MemoryAssociation[]): Promise<void> {
    for (const assoc of associations) {
      try {
        await this.createAssociation(assoc);
      } catch (error) {
        console.error(`Failed to create association:`, error);
      }
    }
  }

  /**
   * Start an episode
   */
  async startEpisode(title: string, context?: string): Promise<{ id: string }> {
    const response = await this.client.post('/v1/episodes/start', {
      title,
      context
    });
    return response.data;
  }

  /**
   * End an episode
   */
  async endEpisode(episodeId: string): Promise<void> {
    await this.client.post(`/v1/episodes/${episodeId}/end`);
  }

  /**
   * Sync all memories and associations to Goldfish
   */
  async sync(
    memories: ParsedMemory[], 
    associations: MemoryAssociation[]
  ): Promise<{ memoriesStored: number; associationsCreated: number }> {
    // Store memories first
    await this.storeMemories(memories);
    
    // Then create associations
    await this.createAssociations(associations);
    
    return {
      memoriesStored: memories.length,
      associationsCreated: associations.length
    };
  }
}
