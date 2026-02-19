/**
 * Graph builder for creating associations between memories
 * Creates relationships based on: shared keywords, temporal proximity, type relations
 */

import { ParsedMemory, MemoryAssociation, RelationType } from './types';

export class GraphBuilder {
  /**
   * Build graph associations between memories
   */
  buildGraph(memories: ParsedMemory[]): MemoryAssociation[] {
    const associations: MemoryAssociation[] = [];
    
    // Build keyword index for fast lookup
    const keywordIndex = this.buildKeywordIndex(memories);
    
    for (let i = 0; i < memories.length; i++) {
      const source = memories[i];
      
      // Find related memories
      for (let j = i + 1; j < memories.length; j++) {
        const target = memories[j];
        
        // Check for various relationship types
        const keywordRelation = this.checkKeywordRelation(source, target, keywordIndex);
        const temporalRelation = this.checkTemporalRelation(source, target);
        const typeRelation = this.checkTypeRelation(source, target);
        
        // Use the strongest relation
        const relation = this.selectStrongestRelation(keywordRelation, temporalRelation, typeRelation);
        
        if (relation) {
          associations.push({
            sourceId: source.id,
            targetId: target.id,
            relation: relation.type,
            weight: relation.weight
          });
        }
      }
    }
    
    return associations;
  }

  /**
   * Build inverted keyword index for efficient lookup
   */
  private buildKeywordIndex(memories: ParsedMemory[]): Map<string, Set<string>> {
    const index = new Map<string, Set<string>>();
    
    for (const memory of memories) {
      const keywords = this.extractKeywords(memory.content);
      
      for (const keyword of keywords) {
        if (!index.has(keyword)) {
          index.set(keyword, new Set());
        }
        index.get(keyword)!.add(memory.id);
      }
    }
    
    return index;
  }

  /**
   * Extract keywords from content (simplified)
   */
  private extractKeywords(content: string): string[] {
    return content
      .toLowerCase()
      .split(/[^\w]+/)
      .filter(word => word.length > 3)
      .filter(word => !this.isStopWord(word));
  }

  /**
   * Common stop words to filter out
   */
  private isStopWord(word: string): boolean {
    const stopWords = new Set([
      'the', 'and', 'that', 'have', 'for', 'not', 'with', 'you', 'this', 'but',
      'his', 'from', 'they', 'she', 'will', 'would', 'there', 'their', 'what',
      'about', 'when', 'make', 'like', 'time', 'just', 'know', 'take', 'people'
    ]);
    return stopWords.has(word);
  }

  /**
   * Check if two memories are related by shared keywords
   */
  private checkKeywordRelation(
    source: ParsedMemory, 
    target: ParsedMemory,
    keywordIndex: Map<string, Set<string>>
  ): { type: RelationType; weight: number } | null {
    const sourceKeywords = this.extractKeywords(source.content);
    const targetKeywords = this.extractKeywords(target.content);
    
    // Calculate overlap
    const intersection = sourceKeywords.filter(k => targetKeywords.includes(k));
    const union = [...new Set([...sourceKeywords, ...targetKeywords])];
    
    if (intersection.length === 0) return null;
    
    // Jaccard similarity
    const similarity = intersection.length / union.length;
    
    if (similarity > 0.5) {
      return { type: 'related_to', weight: similarity };
    } else if (similarity > 0.3) {
      return { type: 'related_to', weight: similarity * 0.7 };
    }
    
    return null;
  }

  /**
   * Check if two memories are related by temporal proximity
   */
  private checkTemporalRelation(
    source: ParsedMemory, 
    target: ParsedMemory
  ): { type: RelationType; weight: number } | null {
    const sourceTime = source.timestamp.getTime();
    const targetTime = target.timestamp.getTime();
    
    // Calculate time difference in hours
    const diffHours = Math.abs(sourceTime - targetTime) / (1000 * 60 * 60);
    
    if (diffHours < 1) {
      // Same session - strong relation
      return { type: 'part_of', weight: 0.8 };
    } else if (diffHours < 24) {
      // Same day - moderate relation
      return { type: 'related_to', weight: 0.5 };
    } else if (diffHours < 168) { // 1 week
      // Same week - weak relation
      return { type: 'related_to', weight: 0.3 };
    }
    
    return null;
  }

  /**
   * Check if two memories are related by type
   */
  private checkTypeRelation(
    source: ParsedMemory, 
    target: ParsedMemory
  ): { type: RelationType; weight: number } | null {
    // Goal -> Decision chain
    if (source.type === 'goal' && target.type === 'decision') {
      if (this.contentSimilarity(source.content, target.content) > 0.3) {
        return { type: 'caused_by', weight: 0.7 };
      }
    }
    
    // Decision -> Fact (implementation)
    if (source.type === 'decision' && target.type === 'fact') {
      if (this.contentSimilarity(source.content, target.content) > 0.2) {
        return { type: 'caused_by', weight: 0.6 };
      }
    }
    
    // Preference -> Decision influence
    if (source.type === 'preference' && target.type === 'decision') {
      return { type: 'caused_by', weight: 0.5 };
    }
    
    return null;
  }

  /**
   * Calculate simple content similarity
   */
  private contentSimilarity(content1: string, content2: string): number {
    const words1 = new Set(this.extractKeywords(content1));
    const words2 = new Set(this.extractKeywords(content2));
    
    const intersection = new Set([...words1].filter(x => words2.has(x)));
    const union = new Set([...words1, ...words2]);
    
    return intersection.size / union.size;
  }

  /**
   * Select the strongest relation from multiple candidates
   */
  private selectStrongestRelation(
    ...relations: ({
      type: RelationType;
      weight: number;
    } | null)[]
  ): { type: RelationType; weight: number } | null {
    return relations
      .filter((r): r is { type: RelationType; weight: number } => r !== null)
      .sort((a, b) => b.weight - a.weight)[0] || null;
  }

  /**
   * Get all neighbors of a memory up to a certain depth
   */
  getNeighbors(
    memoryId: string, 
    associations: MemoryAssociation[], 
    depth: number = 1
  ): Set<string> {
    const neighbors = new Set<string>();
    const visited = new Set<string>();
    
    const queue: Array<{ id: string; currentDepth: number }> = [
      { id: memoryId, currentDepth: 0 }
    ];
    
    while (queue.length > 0) {
      const { id, currentDepth } = queue.shift()!;
      
      if (visited.has(id) || currentDepth > depth) continue;
      visited.add(id);
      
      // Find all associations involving this memory
      for (const assoc of associations) {
        if (assoc.sourceId === id && !visited.has(assoc.targetId)) {
          neighbors.add(assoc.targetId);
          queue.push({ id: assoc.targetId, currentDepth: currentDepth + 1 });
        } else if (assoc.targetId === id && !visited.has(assoc.sourceId)) {
          neighbors.add(assoc.sourceId);
          queue.push({ id: assoc.sourceId, currentDepth: currentDepth + 1 });
        }
      }
    }
    
    return neighbors;
  }
}
