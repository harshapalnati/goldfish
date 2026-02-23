-- Migration: Add User Summaries Table
-- For Rolling Summary (User Portrait) feature

CREATE TABLE IF NOT EXISTS user_summaries (
    id TEXT PRIMARY KEY,
    user_id TEXT DEFAULT NULL,
    summary_text TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    memory_count INTEGER NOT NULL DEFAULT 0,
    importance_threshold REAL NOT NULL DEFAULT 0.6,
    included_types TEXT NOT NULL DEFAULT '["Identity","Preference","Goal","Decision"]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for faster user lookups
CREATE INDEX IF NOT EXISTS idx_user_summaries_user_id 
ON user_summaries(user_id);

-- Add redundancy tracking to memories table
ALTER TABLE memories ADD COLUMN IF NOT EXISTS is_duplicate INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memories ADD COLUMN IF NOT EXISTS original_memory_id TEXT DEFAULT NULL;

-- Index for duplicate lookups
CREATE INDEX IF NOT EXISTS idx_memories_original_id 
ON memories(original_memory_id);
