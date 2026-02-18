/**
 * Goldfish Memory API Client for JavaScript/TypeScript
 * 
 * Usage:
 * const client = new GoldfishClient('http://localhost:3000');
 * await client.remember('User likes Python', 'preference', 0.9);
 * const results = await client.recall('programming');
 * 
 * @author Harsha Palnati <harshapalnati@gmail.com>
 */

class GoldfishClient {
  /**
   * Create a new Goldfish client
   * @param {string} baseUrl - API base URL (default: http://localhost:3000)
   */
  constructor(baseUrl = 'http://localhost:3000') {
    this.baseUrl = baseUrl;
  }

  /**
   * Store a memory
   * @param {string} content - Memory content
   * @param {string} type - Memory type (fact, preference, goal, decision, event, identity)
   * @param {number} importance - Importance score (0.0 - 1.0)
   * @param {string} source - Source of the memory
   * @returns {Promise<Object>} Stored memory
   */
  async remember(content, type = 'fact', importance = null, source = null) {
    const data = {
      content,
      type,
    };

    if (importance !== null) {
      data.importance = importance;
    }

    if (source !== null) {
      data.source = source;
    }

    const response = await fetch(`${this.baseUrl}/v1/memory`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      throw new Error(`Failed to store memory: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get a memory by ID
   * @param {string} id - Memory ID
   * @returns {Promise<Object>} Memory
   */
  async get(id) {
    const response = await fetch(`${this.baseUrl}/v1/memory/${id}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get memory: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Search memories with hybrid ranking
   * @param {string} query - Search query
   * @param {number} limit - Maximum results
   * @returns {Promise<Array>} Search results
   */
  async recall(query, limit = 10) {
    const response = await fetch(`${this.baseUrl}/v1/search`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query, limit }),
    });

    if (!response.ok) {
      throw new Error(`Failed to search: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Build LLM context with citations
   * @param {string} query - Context query
   * @param {number} tokenBudget - Maximum tokens
   * @returns {Promise<Object>} Context with citations
   */
  async context(query, tokenBudget = 2000) {
    const response = await fetch(`${this.baseUrl}/v1/context`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query, token_budget: tokenBudget }),
    });

    if (!response.ok) {
      throw new Error(`Failed to build context: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Start an episode
   * @param {string} title - Episode title
   * @param {string} context - Episode context
   * @returns {Promise<Object>} Episode
   */
  async startEpisode(title, context = '') {
    const response = await fetch(`${this.baseUrl}/v1/episodes/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ title, context }),
    });

    if (!response.ok) {
      throw new Error(`Failed to start episode: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * End an episode
   * @param {string} id - Episode ID
   * @returns {Promise<void>}
   */
  async endEpisode(id) {
    const response = await fetch(`${this.baseUrl}/v1/episodes/${id}/end`, {
      method: 'POST',
    });

    if (!response.ok) {
      throw new Error(`Failed to end episode: ${response.statusText}`);
    }
  }

  /**
   * Check health
   * @returns {Promise<Object>} Health status
   */
  async health() {
    const response = await fetch(`${this.baseUrl}/health`);
    return response.json();
  }
}

// Export for both CommonJS and ES modules
if (typeof module !== 'undefined' && module.exports) {
  module.exports = GoldfishClient;
}

// Example usage
async function demo() {
  const client = new GoldfishClient();
  
  // Store memories
  await client.remember(
    'User prefers dark mode in all applications',
    'preference',
    0.9
  );
  
  await client.remember(
    'Project deadline is March 15th',
    'fact',
    0.8
  );
  
  // Search
  const results = await client.recall('user preferences', 5);
  console.log('Search results:', results);
  
  // Build context
  const ctx = await client.context('What should I know?', 500);
  console.log('Context:', ctx.context);
  console.log('Citations:', ctx.citations);
}

// Run demo if executed directly
if (typeof require !== 'undefined' && require.main === module) {
  demo().catch(console.error);
}
