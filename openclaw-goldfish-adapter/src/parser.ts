/**
 * Markdown parser for OpenClaw memory files
 * Parses MEMORY.md and memory/*.md files into structured memories
 */

import * as fs from 'fs';
import * as path from 'path';
import { glob } from 'glob';
import { 
  ParsedMemory, 
  MemoryType, 
  OpenClawMemoryFile, 
  OpenClawChunk 
} from './types';
import { v4 as uuidv4 } from 'uuid';

export class MarkdownParser {
  private workspaceDir: string;
  private memoryFile: string;
  private memoryPattern: string;

  constructor(workspaceDir: string, memoryFile = 'MEMORY.md', memoryPattern = 'memory/**/*.md') {
    this.workspaceDir = workspaceDir;
    this.memoryFile = memoryFile;
    this.memoryPattern = memoryPattern;
  }

  /**
   * Parse all memory files in the workspace
   */
  async parseAllMemories(): Promise<ParsedMemory[]> {
    const memories: ParsedMemory[] = [];

    // Parse main MEMORY.md file
    const mainMemoryPath = path.join(this.workspaceDir, this.memoryFile);
    if (fs.existsSync(mainMemoryPath)) {
      const mainMemories = await this.parseMemoryFile(mainMemoryPath);
      memories.push(...mainMemories);
    }

    // Parse memory/*.md files
    const memoryFiles = await glob(this.memoryPattern, {
      cwd: this.workspaceDir,
      absolute: true
    });

    for (const file of memoryFiles) {
      const fileMemories = await this.parseMemoryFile(file);
      memories.push(...fileMemories);
    }

    return memories;
  }

  /**
   * Parse a single memory file
   */
  async parseMemoryFile(filePath: string): Promise<ParsedMemory[]> {
    const content = fs.readFileSync(filePath, 'utf-8');
    const chunks = this.extractChunks(content);
    
    return chunks.map(chunk => this.chunkToMemory(chunk, filePath));
  }

  /**
   * Extract memory chunks from markdown content
   * OpenClaw uses 400-token chunks with 80-token overlap
   * For simplicity, we extract by sections (## headers) and bullet points
   */
  private extractChunks(content: string): OpenClawChunk[] {
    const chunks: OpenClawChunk[] = [];
    
    // Split by headers
    const sections = content.split(/^#{2,3}\s+/m);
    
    for (const section of sections) {
      if (!section.trim()) continue;
      
      const lines = section.split('\n');
      const firstLine = lines[0].trim();
      const restContent = lines.slice(1).join('\n').trim();
      
      // Extract bullet points as separate memories
      const bulletMatches = restContent.match(/^[-*]\s+(.+)$/gm);
      if (bulletMatches) {
        for (const bullet of bulletMatches) {
          const bulletContent = bullet.replace(/^[-*]\s+/, '').trim();
          if (bulletContent.length > 10) {
            chunks.push({
              content: bulletContent,
              timestamp: this.extractTimestamp(bulletContent),
              tags: this.extractTags(firstLine)
            });
          }
        }
      }
      
      // Also add the full section as a memory if it's substantial
      if (restContent.length > 50) {
        chunks.push({
          content: `${firstLine}\n${restContent}`,
          timestamp: this.extractTimestamp(restContent),
          tags: this.extractTags(firstLine)
        });
      }
    }

    // If no structured sections found, treat entire content as one memory
    if (chunks.length === 0 && content.trim().length > 20) {
      chunks.push({
        content: content.trim(),
        timestamp: this.extractTimestamp(content),
        tags: []
      });
    }

    return chunks;
  }

  /**
   * Convert a chunk to a parsed memory with type detection
   */
  private chunkToMemory(chunk: OpenClawChunk, source: string): ParsedMemory {
    const content = chunk.content;
    const type = this.inferMemoryType(content);
    const importance = this.calculateImportance(content, type);
    
    return {
      id: uuidv4(),
      content: content.slice(0, 1000), // Limit length
      type,
      importance,
      timestamp: chunk.timestamp || new Date(),
      metadata: {
        source,
        chunkTags: chunk.tags || []
      },
      source,
      tags: chunk.tags || []
    };
  }

  /**
   * Infer memory type from content
   */
  private inferMemoryType(content: string): MemoryType {
    const lower = content.toLowerCase();
    
    // Check for explicit type markers
    if (lower.match(/^(user['']s?\s+)?name\s+(is|=)/)) return 'identity';
    if (lower.match(/^(user['']s?\s+)?(location|lives\s+in)/)) return 'identity';
    if (lower.match(/^(user['']s?\s+)?(job|works\s+as)/)) return 'identity';
    if (lower.includes('goal:') || lower.includes('want to') || lower.includes('need to')) return 'goal';
    if (lower.includes('decision:') || lower.includes('decided to') || lower.includes('choose')) return 'decision';
    if (lower.match(/\b(prefer|likes?|enjoy|favorite|hates?)\b/)) return 'preference';
    if (lower.match(/\b(last week|yesterday|today|last month|recently)\b/)) return 'event';
    if (lower.match(/\b(is|are|was|were)\s+\w+\b/)) return 'fact';
    
    return 'observation';
  }

  /**
   * Calculate importance based on type and content
   */
  private calculateImportance(content: string, type: MemoryType): number {
    const baseImportance: Record<MemoryType, number> = {
      'identity': 1.0,
      'goal': 0.9,
      'decision': 0.8,
      'preference': 0.7,
      'fact': 0.6,
      'event': 0.4,
      'observation': 0.3
    };

    let importance = baseImportance[type] || 0.5;
    
    // Boost for critical keywords
    const lower = content.toLowerCase();
    if (lower.includes('critical') || lower.includes('important') || lower.includes('never')) {
      importance += 0.1;
    }
    
    // Cap at 1.0
    return Math.min(importance, 1.0);
  }

  /**
   * Extract timestamp from content if present
   */
  private extractTimestamp(content: string): Date | undefined {
    // Match common date patterns
    const patterns = [
      /(\d{4}-\d{2}-\d{2})/,
      /(last week|yesterday|today|last month|recently)/i,
      /(\d{1,2}\/\d{1,2}\/\d{2,4})/
    ];

    for (const pattern of patterns) {
      const match = content.match(pattern);
      if (match) {
        const date = new Date();
        const text = match[1].toLowerCase();
        
        if (text === 'last week') date.setDate(date.getDate() - 7);
        else if (text === 'yesterday') date.setDate(date.getDate() - 1);
        else if (text === 'today') return date;
        else if (text === 'last month') date.setMonth(date.getMonth() - 1);
        else return new Date(match[1]);
        
        return date;
      }
    }

    return undefined;
  }

  /**
   * Extract tags from section header
   */
  private extractTags(header: string): string[] {
    const tags: string[] = [];
    const lower = header.toLowerCase();
    
    if (lower.includes('preference')) tags.push('preference');
    if (lower.includes('goal')) tags.push('goal');
    if (lower.includes('decision')) tags.push('decision');
    if (lower.includes('fact')) tags.push('fact');
    if (lower.includes('event')) tags.push('event');
    if (lower.includes('identity')) tags.push('identity');
    
    return tags;
  }

  /**
   * Watch for file changes and return new/modified files
   */
  watchForChanges(callback: (file: string) => void): void {
    const watchPaths = [
      path.join(this.workspaceDir, this.memoryFile),
      path.join(this.workspaceDir, 'memory')
    ];

    for (const watchPath of watchPaths) {
      if (fs.existsSync(watchPath)) {
        fs.watch(watchPath, { recursive: true }, (eventType, filename) => {
          if (filename && filename.endsWith('.md')) {
            callback(path.join(watchPath, filename));
          }
        });
      }
    }
  }
}
