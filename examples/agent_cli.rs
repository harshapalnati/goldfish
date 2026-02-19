//! Interactive agent CLI backed by Goldfish MemoryCortex.
//!
//! Run:
//!   cargo run --example agent_cli

use anyhow::Result;
use goldfish::{ContextWindow, Memory, MemoryCortex, MemoryType};
use std::io::{self, Write};

struct CliAgent {
    name: String,
    cortex: MemoryCortex,
}

impl CliAgent {
    async fn new(name: &str, data_dir: &str) -> Result<Self> {
        let cortex = MemoryCortex::new(data_dir).await?;

        let directive =
            Memory::new("Be concise, accurate, and action-oriented.", MemoryType::Identity)
                .with_importance(1.0);
        cortex.remember(&directive).await?;
        cortex.pin(&directive.id).await;

        Ok(Self {
            name: name.to_string(),
            cortex,
        })
    }

    async fn handle_message(&self, message: &str) -> Result<()> {
        let recalled = self.cortex.recall(message, 3).await?;
        if recalled.is_empty() {
            println!("agent> No relevant memories found yet.");
        } else {
            println!("agent> Relevant memories:");
            for hit in &recalled {
                println!("  - [{:.2}] {}", hit.score, hit.memory.content);
                self.cortex.think_about(&hit.memory.id).await?;
            }
        }

        let lower = message.to_lowercase();
        if contains_any(
            &lower,
            &["i prefer", "i like", "i love", "favorite", "prefer"],
        ) {
            self.cortex.prefer(message, 0.8).await?;
            println!("agent> Saved as a preference.");
        } else if contains_any(
            &lower,
            &["my goal", "i need to", "i want to", "i plan to", "goal"],
        ) {
            self.cortex.goal(message).await?;
            println!("agent> Saved as a goal.");
        } else if contains_any(&lower, &["todo", "to do", "remind me"]) {
            let todo = Memory::new(message, MemoryType::Todo).with_importance(0.85);
            self.cortex.remember(&todo).await?;
            println!("agent> Saved as a todo.");
        } else {
            let fact = Memory::new(message, MemoryType::Fact);
            self.cortex.remember(&fact).await?;
            println!("agent> Saved as a fact.");
        }

        println!("agent> Stored. Ask `/context` to see current prompt context.");
        Ok(())
    }

    async fn show_context(&self) -> Result<()> {
        let config = ContextWindow {
            max_tokens: 800,
            include_working_memory: true,
            include_experience: true,
            include_important: true,
            max_important: 8,
        };
        let context = self.cortex.build_context(&config).await?;
        println!("\n----- LLM Context -----\n{}\n-----------------------", context);
        Ok(())
    }

    async fn show_goals(&self) -> Result<()> {
        let goals = self.cortex.get_goals().await?;
        println!("\nGoals:");
        if goals.is_empty() {
            println!("  (none)");
        } else {
            for g in goals {
                println!("  - {}", g.content);
            }
        }
        Ok(())
    }

    async fn show_important(&self) -> Result<()> {
        let important = self.cortex.get_important(5).await?;
        println!("\nTop important memories:");
        if important.is_empty() {
            println!("  (none)");
        } else {
            for m in important {
                println!(
                    "  - [{}] {} (importance {:.2})",
                    m.memory_type, m.content, m.importance
                );
            }
        }
        Ok(())
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn print_help() {
    println!("Commands:");
    println!("  /help       Show this help");
    println!("  /context    Build LLM-ready context");
    println!("  /goals      List learned goals");
    println!("  /important  List top important memories");
    println!("  /quit       Exit");
}

#[tokio::main]
async fn main() -> Result<()> {
    let agent = CliAgent::new("GoldfishCLI", "./agent_cli_data").await?;

    println!("Goldfish interactive agent: {}", agent.name);
    println!("Type normal chat lines to store/retrieve memory.");
    print_help();

    agent
        .cortex
        .start_episode("Interactive session", "CLI agent session")
        .await?;

    let mut line = String::new();
    loop {
        line.clear();
        print!("\nyou> ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut line)?;

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" => break,
            "/help" => print_help(),
            "/context" => agent.show_context().await?,
            "/goals" => agent.show_goals().await?,
            "/important" => agent.show_important().await?,
            _ => agent.handle_message(input).await?,
        }
    }

    if let Some(episode) = agent.cortex.end_episode().await? {
        println!(
            "\nSession closed. Episode '{}' lasted {:?}.",
            episode.title,
            episode.duration()
        );
    }

    Ok(())
}
