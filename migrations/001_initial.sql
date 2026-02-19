-- Migration: Initial schema

-- Memories table
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    last_accessed_at TIMESTAMP NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    source TEXT,
    session_id TEXT,
    forgotten BOOLEAN NOT NULL DEFAULT 0,
    metadata TEXT,
    -- Confidence scoring (JSON stored confidence data)
    confidence_score REAL NOT NULL DEFAULT 0.5,
    confidence_data TEXT,  -- JSON serialized MemoryConfidence
    verification_status TEXT NOT NULL DEFAULT 'unverified'
);

-- Associations (graph edges)
CREATE TABLE associations (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 0.5,
    created_at TIMESTAMP NOT NULL
);

-- Indexes for performance
CREATE INDEX idx_memories_type ON memories(memory_type);
CREATE INDEX idx_memories_importance ON memories(importance DESC);
CREATE INDEX idx_memories_created ON memories(created_at DESC);
CREATE INDEX idx_memories_forgotten ON memories(forgotten);
CREATE INDEX idx_memories_session ON memories(session_id);
CREATE INDEX idx_memories_confidence ON memories(confidence_score DESC);
CREATE INDEX idx_memories_verification ON memories(verification_status);

CREATE INDEX idx_associations_source ON associations(source_id);
CREATE INDEX idx_associations_target ON associations(target_id);
CREATE INDEX idx_associations_type ON associations(relation_type);

-- Unique constraint to prevent duplicate associations
CREATE UNIQUE INDEX idx_associations_unique 
ON associations(source_id, target_id, relation_type);
