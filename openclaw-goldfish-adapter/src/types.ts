/**
 * Types for OpenClaw ↔ Goldfish integration
 */

export interface ParsedMemory {
  id: string;
  content: string;
  type: MemoryType;
  importance: number;
  timestamp: Date;
  metadata: Record<string, any>;
  source: string;
  tags: string[];
}

export type MemoryType = 
  | 'fact' 
  | 'preference' 
  | 'goal' 
  | 'decision' 
  | 'event' 
  | 'identity' 
  | 'observation';

export interface MemoryAssociation {
  sourceId: string;
  targetId: string;
  relation: RelationType;
  weight: number;
}

export type RelationType = 
  | 'related_to' 
  | 'caused_by' 
  | 'part_of' 
  | 'updates' 
  | 'contradicts';

export interface GoldfishConfig {
  baseUrl: string;
  timeout?: number;
  retries?: number;
}

export interface OpenClawConfig {
  workspaceDir: string;
  memoryFile?: string;
  memoryPattern?: string;
}

export interface AdapterConfig {
  goldfish: GoldfishConfig;
  openclaw: OpenClawConfig;
  enableGraph?: boolean;
  autoSync?: boolean;
  syncInterval?: number;
}

export interface SearchResult {
  id: string;
  content: string;
  type: MemoryType;
  score: number;
  metadata?: Record<string, any>;
}

export interface AgentContext {
  agentId: string;
  currentTask?: string;
  lastUserMessage?: string;
  systemPrompt: string;
  conversation: string[];
  metadata: Record<string, any>;
}

export interface LifecycleHooks {
  beforeAgentStart?: (context: AgentContext) => Promise<void>;
  afterAgentEnd?: (context: AgentContext) => Promise<void>;
  onMemoryAdded?: (memory: ParsedMemory) => Promise<void>;
}

// OpenClaw specific types
export interface OpenClawMemoryFile {
  path: string;
  content: string;
  lastModified: Date;
}

export interface OpenClawChunk {
  content: string;
  timestamp?: Date;
  tags?: string[];
}
