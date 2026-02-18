#!/usr/bin/env python3
"""
Goldfish Python Client - Simple as Supermemory

Install: pip install requests
Usage: python goldfish_client.py
"""

import requests
import json

BASE_URL = "http://localhost:3000/v1"

class GoldfishClient:
    """Simple client for Goldfish Memory API"""
    
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url
    
    def remember(self, content: str, memory_type: str = "fact", importance: float = None, source: str = None):
        """Store a memory"""
        data = {
            "content": content,
            "type": memory_type,
        }
        if importance:
            data["importance"] = importance
        if source:
            data["source"] = source
        
        resp = requests.post(f"{self.base_url}/v1/memory", json=data)
        resp.raise_for_status()
        return resp.json()
    
    def recall(self, query: str, limit: int = 10):
        """Search memories"""
        data = {"query": query, "limit": limit}
        resp = requests.post(f"{self.base_url}/v1/search", json=data)
        resp.raise_for_status()
        return resp.json()
    
    def context(self, query: str, token_budget: int = 2000):
        """Build context for LLM"""
        data = {"query": query, "token_budget": token_budget}
        resp = requests.post(f"{self.base_url}/v1/context", json=data)
        resp.raise_for_status()
        return resp.json()


def demo():
    """Quick demo of Goldfish capabilities"""
    print("🐠 Goldfish Memory - Python Demo\n")
    
    client = GoldfishClient()
    
    # 1. Store some memories
    print("1. Storing memories...")
    client.remember(
        "User prefers dark mode in all applications",
        memory_type="preference",
        importance=0.9
    )
    client.remember(
        "Project deadline is March 15th",
        memory_type="fact",
        importance=0.8
    )
    client.remember(
        "Use Rust for the backend API",
        memory_type="decision",
        importance=0.85,
        source="architecture-review"
    )
    print("   ✅ Stored 3 memories\n")
    
    # 2. Search
    print("2. Searching for 'preferences'...")
    results = client.recall("preferences", limit=5)
    for r in results:
        print(f"   • [{r['type']}] {r['content'][:50]}... (score: {r['score']:.2f})")
    print()
    
    # 3. Build context for LLM
    print("3. Building LLM context...")
    ctx = client.context("What should I know about this user?", token_budget=500)
    print(f"   Tokens used: {ctx['tokens_used']}")
    print(f"   Memories included: {ctx['memories_included']}")
    print("\n   Context preview:")
    print("   " + "\n   ".join(ctx['context'].split('\n')[:5]))
    print()
    
    print("✅ Demo complete!")


if __name__ == "__main__":
    demo()
