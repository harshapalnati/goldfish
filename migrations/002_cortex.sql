-- Migration: Agentic Memory Cortex tables

-- Experiences (episodic memory)
CREATE TABLE IF NOT EXISTS experiences (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    context TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP,
    importance REAL NOT NULL DEFAULT 0.5
);

-- Join table: experiences <-> memories
CREATE TABLE IF NOT EXISTS experience_memories (
    experience_id TEXT NOT NULL REFERENCES experiences(id) ON DELETE CASCADE,
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (experience_id, memory_id)
);

-- Memory summaries (consolidation)
CREATE TABLE IF NOT EXISTS memory_summaries (
    id TEXT PRIMARY KEY,
    summary_text TEXT NOT NULL,
    original_memory_ids TEXT NOT NULL,  -- JSON array of memory IDs
    memory_type TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_experiences_started ON experiences(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_experiences_importance ON experiences(importance DESC);
CREATE INDEX IF NOT EXISTS idx_experience_memories_exp ON experience_memories(experience_id);
CREATE INDEX IF NOT EXISTS idx_experience_memories_mem ON experience_memories(memory_id);
CREATE INDEX IF NOT EXISTS idx_summaries_type ON memory_summaries(memory_type);
CREATE INDEX IF NOT EXISTS idx_summaries_created ON memory_summaries(created_at DESC);
