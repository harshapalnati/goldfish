//! Example of an AI agent using the MemoryCortex

use goldfish::{
    MemoryCortex, Memory, MemoryType,
};

/// Simulated AI Agent with Memory Cortex
struct Agent {
    name: String,
    cortex: MemoryCortex,
}

impl Agent {
    async fn new(name: &str, data_dir: &str) -> anyhow::Result<Self> {
        let cortex = MemoryCortex::new(data_dir).await?;

        Ok(Self {
            name: name.to_string(),
            cortex,
        })
    }

    /// Process user input and store relevant memories
    async fn process_message(&self, user_message: &str) -> anyhow::Result<String> {
        println!("\n[{}] Processing: '{}'", self.name, user_message);

        // 1. Recall relevant memories
        let relevant = self.cortex.recall(user_message, 5).await?;
        if !relevant.is_empty() {
            println!("  → Recalled {} relevant memories", relevant.len());
            for r in &relevant {
                println!("     - {} (score: {:.2})", r.memory.content, r.score);
            }
        }

        // 2. Think about it (add to working memory)
        for r in &relevant {
            self.cortex.think_about(&r.memory.id).await?;
        }

        // 3. Extract and store memories
        self.learn_from_message(user_message).await?;

        Ok(format!("Processed: {}", user_message))
    }

    /// Learn from conversation
    async fn learn_from_message(&self, message: &str) -> anyhow::Result<()> {
        let msg = message.to_lowercase();
        
        // Preferences
        if msg.contains("like") || msg.contains("prefer") || msg.contains("love") {
            self.cortex.prefer(message, 0.7).await?;
            println!("  → Learned preference");
        }
        
        // Goals
        if msg.contains("goal") || msg.contains("want") || msg.contains("need to") {
            self.cortex.goal(message).await?;
            println!("  → Learned goal");
        }
        
        // Facts
        if msg.contains("is") || msg.contains("are") || msg.contains("was") {
            self.cortex.remember(&Memory::new(message, MemoryType::Fact)).await?;
            println!("  → Learned fact");
        }
        
        Ok(())
    }

    /// Show what the agent knows
    async fn show_knowledge(&self) -> anyhow::Result<()> {
        println!("\n[{}'s Knowledge]", self.name);

        // Goals
        println!("\n🎯 Goals:");
        let goals = self.cortex.get_goals().await?;
        for goal in &goals {
            println!("   • {}", goal.content);
        }

        // Important
        println!("\n⭐ Important:");
        let important = self.cortex.get_important(5).await?;
        for m in &important {
            println!("   • [{}] {}", m.memory_type, &m.content[..m.content.len().min(50)]);
        }

        // Context
        println!("\n🧠 Working Memory:");
        let context = self.cortex.get_context().await;
        for item in &context {
            println!("   • {} (attention: {:.2})", 
                &item.content[..item.content.len().min(30)], 
                item.attention_score
            );
        }

        Ok(())
    }

    /// Get full context for LLM
    async fn get_llm_context(&self) -> Result<String, goldfish::MemoryError> {
        use goldfish::ContextWindow;
        
        let config = ContextWindow {
            max_tokens: 2000,
            include_working_memory: true,
            include_experience: true,
            include_important: true,
            max_important: 10,
        };
        
        self.cortex.build_context(&config).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== AI Agent with Memory Cortex ===\n");

    // Create agent
    let agent = Agent::new("GoldfishBot", "./agent_cortex_data").await?;
    println!("✓ Agent '{}' initialized\n", agent.name);

    // Start a new episode
    println!("🎬 Starting episode: 'Initial Interaction'...");
    agent.cortex.start_episode("Initial Interaction", "User onboarding session").await?;

    // Pin a core directive
    let directive = Memory::new("Always be helpful and concise", MemoryType::Identity)
        .with_importance(1.0);
    agent.cortex.remember(&directive).await?;
    agent.cortex.pin(&directive.id).await;
    println!("📌 Pinned directive: {}", directive.content);

    // Simulate conversation
    let conversations = vec![
        "I like working with Python and Rust",
        "My goal is to build an intelligent assistant",
        "I prefer clean code over clever code",
        "The project deadline is next Friday",
        "Use dark theme in the IDE",
    ];

    for message in &conversations {
        agent.process_message(message).await?;
    }

    // End episode
    let episode = agent.cortex.end_episode().await?.unwrap();
    println!("\n✅ Episode ended. Duration: {:?}", episode.duration());

    // Show what agent learned
    agent.show_knowledge().await?;

    // Get LLM context
    println!("\n📋 Context for LLM:");
    println!("{}", agent.get_llm_context().await?);

    println!("\n=== Example complete ===");
    Ok(())
}
