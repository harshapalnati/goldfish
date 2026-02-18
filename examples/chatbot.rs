//! Chatbot example showing conversational memory

use goldfish::{Memory, MemorySystem, MemoryType, SearchConfig, SearchMode};
use std::io::{self, Write};

/// Simple chatbot with memory
struct Chatbot {
    memory: MemorySystem,
    session_id: String,
    user_name: Option<String>,
}

impl Chatbot {
    async fn new(data_dir: &str) -> anyhow::Result<Self> {
        let memory = MemorySystem::new(data_dir).await?;
        let session_id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            memory,
            session_id,
            user_name: None,
        })
    }

    async fn chat(&mut self, user_input: &str) -> anyhow::Result<String> {
        // Extract user name if mentioned
        if user_input.to_lowercase().contains("my name is") {
            let name = user_input
                .to_lowercase()
                .split("my name is")
                .nth(1)
                .unwrap_or("")
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            if !name.is_empty() {
                self.user_name = Some(name.clone());

                let identity =
                    Memory::new(format!("User's name is {}", name), MemoryType::Identity)
                        .with_session_id(&self.session_id);

                self.memory.save(&identity).await?;
            }
        }

        // Recall relevant context
        let context = self.get_context(user_input).await?;

        // Generate response based on context
        let response = self.generate_response(user_input, &context).await;

        // Store this exchange
        self.store_exchange(user_input, &response).await?;

        Ok(response)
    }

    async fn get_context(&self, query: &str) -> anyhow::Result<Vec<String>> {
        let config = SearchConfig {
            mode: SearchMode::FullText,
            max_results: 10,
            ..Default::default()
        };

        let results = self.memory.search_with_config(query, &config).await?;
        let curated: Vec<_> = results.into_iter().take(5).collect();

        Ok(curated
            .into_iter()
            .map(|r| format!("[{}] {}", r.memory.memory_type, r.memory.content))
            .collect())
    }

    async fn generate_response(&self, user_input: &str, context: &[String]) -> String {
        let name = self.user_name.as_deref().unwrap_or("friend");

        // Simple rule-based responses (replace with LLM in production)
        if user_input.to_lowercase().contains("hello") || user_input.to_lowercase().contains("hi") {
            return format!("Hello {}! How can I help you today?", name);
        }

        if user_input.to_lowercase().contains("what do you know") {
            if context.is_empty() {
                return format!(
                    "I'm just getting to know you, {}. Tell me about yourself!",
                    name
                );
            } else {
                let mut response = format!("Here's what I remember about you, {}:\n", name);
                for item in context.iter().take(3) {
                    response.push_str(&format!("  • {}\n", item));
                }
                return response;
            }
        }

        if user_input.to_lowercase().contains("forget") {
            return "I can forget things if you ask me to. Which memory should I forget?"
                .to_string();
        }

        // Default response using context
        if !context.is_empty() {
            return format!(
                "Based on what I know about you, {}: {}. What else would you like to discuss?",
                name, context[0]
            );
        }

        format!(
            "Interesting! I'm learning more about you, {}. Tell me more.",
            name
        )
    }

    async fn store_exchange(&self, user_input: &str, _bot_response: &str) -> anyhow::Result<()> {
        // Store user message as observation
        let observation = Memory::new(
            format!("User said: {}", user_input),
            MemoryType::Observation,
        )
        .with_session_id(&self.session_id)
        .with_importance(0.4);

        self.memory.save(&observation).await?;

        // Extract facts, preferences, etc.
        if user_input.to_lowercase().contains("i like")
            || user_input.to_lowercase().contains("i love")
            || user_input.to_lowercase().contains("i prefer")
        {
            let pref = Memory::new(user_input.to_string(), MemoryType::Preference)
                .with_session_id(&self.session_id);
            self.memory.save(&pref).await?;
        }

        if user_input.to_lowercase().contains("i am") || user_input.to_lowercase().contains("i'm") {
            let fact = Memory::new(user_input.to_string(), MemoryType::Fact)
                .with_session_id(&self.session_id);
            self.memory.save(&fact).await?;
        }

        Ok(())
    }

    async fn show_memories(&self) -> anyhow::Result<()> {
        println!("\n=== Bot's Memory ===");

        let all_types = vec![
            MemoryType::Identity,
            MemoryType::Goal,
            MemoryType::Preference,
            MemoryType::Fact,
            MemoryType::Observation,
        ];

        for mem_type in all_types {
            let memories = self.memory.get_by_type(mem_type, 5).await?;
            if !memories.is_empty() {
                println!("\n{:?}:", mem_type);
                for mem in memories {
                    println!("  • {}", mem.content);
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Chatbot with Memory ===\n");
    println!("Type 'quit' to exit, 'memories' to see what I remember\n");

    let mut bot = Chatbot::new("./data/chatbot_example").await?;

    // Pre-populate with some example interactions
    println!("(Pre-populating with example data...)\n");

    let examples = vec![
        ("My name is Alice", MemoryType::Identity),
        ("I like hiking in the mountains", MemoryType::Preference),
        ("I'm a software engineer", MemoryType::Fact),
        ("I want to learn Rust", MemoryType::Goal),
    ];

    for (content, mem_type) in examples {
        let memory = Memory::new(content, mem_type).with_session_id(&bot.session_id);
        bot.memory.save(&memory).await?;
    }

    // Interactive loop
    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\nGoodbye!");
            break;
        }

        if input.eq_ignore_ascii_case("memories") {
            bot.show_memories().await?;
            continue;
        }

        if input.is_empty() {
            continue;
        }

        let response = bot.chat(input).await?;
        println!("Bot: {}\n", response);
    }

    Ok(())
}
