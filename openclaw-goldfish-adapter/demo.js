#!/usr/bin/env node
/**
 * Demo: OpenClaw + Goldfish Integration
 * 
 * This script demonstrates how the adapter works by:
 * 1. Starting Goldfish server
 * 2. Creating sample OpenClaw memory files
 * 3. Syncing to Goldfish
 * 4. Searching and building context
 * 5. Extracting new memories from conversation
 */

const axios = require('axios');
const fs = require('fs');
const path = require('path');

const GOLDFISH_URL = 'http://localhost:3000';
const WORKSPACE_DIR = './demo_workspace';

// Sample OpenClaw memory content
const SAMPLE_MEMORY_MD = `# User Memory

## Identity
- User's name is Alex
- User works as a software engineer at a startup
- User lives in San Francisco, California

## Preferences
- User prefers dark mode in all applications
- User likes coffee with oat milk
- User prefers Slack over email for work communication
- User likes hiking on weekends

## Goals
- Goal: Learn Rust programming language this year
- Goal: Build a side project and launch it
- Goal: Get AWS certification by end of quarter

## Decisions
- Decision: Use SQLite for local storage instead of PostgreSQL
- Decision: Adopt Docker for all deployment scenarios
`;

const SAMPLE_CONVERSATION = [
  'User: Hey, can you help me build a Rust API?',
  'AI: Absolutely! What kind of API are you looking to build?',
  'User: I want to create a memory system for AI agents.',
  'AI: That sounds interesting! Are you planning to use any specific database?',
  'User: I prefer SQLite for simplicity.',
  'AI: Great choice! SQLite is lightweight and perfect for local storage.',
  'User: Yes, and I want to use Docker for deployment.',
  'AI: Excellent! Docker will make deployment much easier.'
];

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function checkGoldfish() {
  try {
    const response = await axios.get(`${GOLDFISH_URL}/health`, { timeout: 1000 });
    console.log('✅ Goldfish server is running');
    return true;
  } catch (error) {
    console.log('❌ Goldfish server is not running');
    console.log('   Please start it with: cargo run --example server --features dashboard');
    return false;
  }
}

async function setupWorkspace() {
  console.log('\n📁 Setting up demo workspace...');
  
  // Create workspace directory
  if (!fs.existsSync(WORKSPACE_DIR)) {
    fs.mkdirSync(WORKSPACE_DIR, { recursive: true });
  }
  
  // Create memory directory
  const memoryDir = path.join(WORKSPACE_DIR, 'memory');
  if (!fs.existsSync(memoryDir)) {
    fs.mkdirSync(memoryDir, { recursive: true });
  }
  
  // Write MEMORY.md
  fs.writeFileSync(path.join(WORKSPACE_DIR, 'MEMORY.md'), SAMPLE_MEMORY_MD);
  
  console.log('✅ Created sample OpenClaw memory files');
}

async function parseMemories() {
  console.log('\n📄 Parsing OpenClaw memory files...');
  
  const content = fs.readFileSync(path.join(WORKSPACE_DIR, 'MEMORY.md'), 'utf-8');
  const memories = [];
  
  // Simple parsing (bullet points)
  const lines = content.split('\n');
  for (const line of lines) {
    if (line.match(/^[-*]\s+(.+)$/)) {
      const memContent = line.replace(/^[-*]\s+/, '').trim();
      if (memContent.length > 10) {
        // Infer type
        let type = 'fact';
        const lower = memContent.toLowerCase();
        if (lower.includes('name') || lower.includes('lives') || lower.includes('works')) {
          type = 'identity';
        } else if (lower.includes('goal:')) {
          type = 'goal';
        } else if (lower.includes('decision:')) {
          type = 'decision';
        } else if (lower.match(/\b(prefer|likes?)\b/)) {
          type = 'preference';
        }
        
        // Calculate importance
        const importanceMap = {
          'identity': 1.0,
          'goal': 0.9,
          'decision': 0.8,
          'preference': 0.7,
          'fact': 0.6
        };
        
        memories.push({
          id: `mem_${Date.now()}_${memories.length}`,
          content: memContent,
          type,
          importance: importanceMap[type] || 0.5,
          source: 'MEMORY.md'
        });
      }
    }
  }
  
  console.log(`✅ Parsed ${memories.length} memories`);
  return memories;
}

async function syncToGoldfish(memories) {
  console.log('\n🔄 Syncing memories to Goldfish...');
  
  for (const memory of memories) {
    try {
      await axios.post(`${GOLDFISH_URL}/v1/memory`, {
        content: memory.content,
        type: memory.type,
        importance: memory.importance,
        source: memory.source
      });
      process.stdout.write('.');
    } catch (error) {
      console.error(`\n❌ Failed to store memory: ${error.message}`);
    }
  }
  
  console.log(`\n✅ Synced ${memories.length} memories to Goldfish`);
}

async function simulateBeforeAgentStart() {
  console.log('\n🎬 Simulating: beforeAgentStart hook');
  console.log('   Agent query: "Build a Rust API with SQLite"');
  
  try {
    const response = await axios.post(`${GOLDFISH_URL}/v1/search`, {
      query: 'rust sqlite api',
      limit: 5
    });
    
    const memories = response.data;
    console.log(`\n📚 Retrieved ${memories.length} relevant memories:`);
    
    for (let i = 0; i < memories.length; i++) {
      const mem = memories[i];
      console.log(`   ${i + 1}. [${mem.type}] ${mem.content.substring(0, 60)}...`);
    }
    
    // Format for system prompt
    console.log('\n💬 Formatted context for LLM:');
    console.log('   ## Relevant Context from Memory');
    memories.forEach((mem, i) => {
      console.log(`   ${i + 1}. [${mem.type}] ${mem.content}`);
    });
    
    return memories;
  } catch (error) {
    console.error('❌ Search failed:', error.message);
    return [];
  }
}

async function simulateAfterAgentEnd() {
  console.log('\n🎬 Simulating: afterAgentEnd hook');
  console.log('   Extracting new memories from conversation...');
  
  const newMemories = [];
  
  // Extract preferences
  if (SAMPLE_CONVERSATION.some(msg => msg.includes('prefer SQLite'))) {
    newMemories.push({
      content: 'User prefers SQLite for local storage',
      type: 'preference',
      importance: 0.7,
      source: 'conversation'
    });
  }
  
  // Extract decisions
  if (SAMPLE_CONVERSATION.some(msg => msg.includes('Docker'))) {
    newMemories.push({
      content: 'User decided to use Docker for deployment',
      type: 'decision',
      importance: 0.8,
      source: 'conversation'
    });
  }
  
  // Extract goal
  if (SAMPLE_CONVERSATION.some(msg => msg.includes('memory system'))) {
    newMemories.push({
      content: 'User wants to build a memory system for AI agents',
      type: 'goal',
      importance: 0.9,
      source: 'conversation'
    });
  }
  
  console.log(`\n📝 Extracted ${newMemories.length} new memories:`);
  newMemories.forEach((mem, i) => {
    console.log(`   ${i + 1}. [${mem.type}] ${mem.content}`);
  });
  
  // Store new memories
  for (const memory of newMemories) {
    try {
      await axios.post(`${GOLDFISH_URL}/v1/memory`, memory);
      process.stdout.write('.');
    } catch (error) {
      console.error(`\n❌ Failed to store memory: ${error.message}`);
    }
  }
  
  console.log(`\n✅ Stored ${newMemories.length} new memories`);
}

async function demonstrateContextBuilding() {
  console.log('\n🧠 Demonstrating: Context Building');
  
  try {
    const response = await axios.post(`${GOLDFISH_URL}/v1/search`, {
      query: 'what does user like',
      limit: 5
    });
    
    const memories = response.data;
    console.log(`\n🔍 Search: "what does user like"`);
    console.log(`   Found ${memories.length} results:`);
    
    memories.forEach((mem, i) => {
      const score = mem.score ? `(${mem.score.toFixed(2)})` : '';
      console.log(`   ${i + 1}. ${score} [${mem.type}] ${mem.content}`);
    });
  } catch (error) {
    console.error('❌ Context building failed:', error.message);
  }
}

async function cleanup() {
  console.log('\n🧹 Cleaning up demo workspace...');
  if (fs.existsSync(WORKSPACE_DIR)) {
    fs.rmSync(WORKSPACE_DIR, { recursive: true, force: true });
  }
  console.log('✅ Cleanup complete');
}

async function main() {
  console.log('╔══════════════════════════════════════════════════════════╗');
  console.log('║     OpenClaw + Goldfish Integration Demo                 ║');
  console.log('╚══════════════════════════════════════════════════════════╝\n');
  
  // Check Goldfish is running
  const goldfishRunning = await checkGoldfish();
  if (!goldfishRunning) {
    console.log('\n⚠️  Please start Goldfish first:');
    console.log('   cargo run --example server --features dashboard');
    process.exit(1);
  }
  
  try {
    // Setup
    await setupWorkspace();
    
    // Parse and sync memories
    const memories = await parseMemories();
    await syncToGoldfish(memories);
    
    await sleep(500);
    
    // Simulate lifecycle hooks
    await simulateBeforeAgentStart();
    
    await sleep(500);
    
    await simulateAfterAgentEnd();
    
    await sleep(500);
    
    // Demonstrate context building
    await demonstrateContextBuilding();
    
    // Cleanup
    await cleanup();
    
    console.log('\n╔══════════════════════════════════════════════════════════╗');
    console.log('║     Demo Complete! 🎉                                    ║');
    console.log('╚══════════════════════════════════════════════════════════╝\n');
    
    console.log('📊 Summary:');
    console.log('   ✅ Parsed OpenClaw memory files');
    console.log('   ✅ Synced memories to Goldfish');
    console.log('   ✅ Retrieved relevant memories for agent context');
    console.log('   ✅ Extracted and stored new memories from conversation');
    console.log('   ✅ Demonstrated hybrid search with BM25 + importance\n');
    
  } catch (error) {
    console.error('\n❌ Demo failed:', error.message);
    await cleanup();
    process.exit(1);
  }
}

main();
