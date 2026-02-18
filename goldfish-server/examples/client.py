import requests
import json
import time

BASE_URL = "http://localhost:3000/v1"

def remember(content, memory_type="fact", importance=None):
    url = f"{BASE_URL}/memory"
    payload = {
        "content": content,
        "memory_type": memory_type,
        "importance": importance
    }
    response = requests.post(url, json=payload)
    if response.status_code == 200:
        print(f"✅ Remembered: {content}")
        return response.json()
    else:
        print(f"❌ Failed to remember: {response.text}")
        return None

def search(query, limit=5):
    url = f"{BASE_URL}/search"
    params = {"q": query, "limit": limit}
    response = requests.get(url, params=params)
    if response.status_code == 200:
        results = response.json()
        print(f"🔍 Search results for '{query}':")
        for mem in results:
            print(f"  - [{mem['memory_type']}] {mem['content']} (imp: {mem['importance']:.2f})")
        return results
    else:
        print(f"❌ Search failed: {response.text}")
        return []

def get_context():
    url = f"{BASE_URL}/context"
    response = requests.get(url)
    if response.status_code == 200:
        data = response.json()
        print("\n🧠 Current Context:")
        print(data['formatted_context'])
        return data
    else:
        print(f"❌ Failed to get context: {response.text}")
        return None

if __name__ == "__main__":
    print("Testing Goldfish Server...")
    
    # 1. Add some memories
    remember("User is a software engineer using Rust", "fact", 0.8)
    remember("Project deadline is next Friday", "goal", 0.9)
    remember("User prefers dark mode", "preference")
    
    # 2. Search
    search("rust")
    
    # 3. Get Context (Agnetic View)
    get_context()
