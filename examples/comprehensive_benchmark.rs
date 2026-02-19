//! Comprehensive Benchmark with Large Dataset (100+ memories, 30+ queries)
//! 
//! This benchmark tests Goldfish retrieval with a realistic, large dataset
//! to measure performance at scale and identify optimization opportunities.

use goldfish::{Memory, MemoryType, MemoryCortex};
use goldfish::eval_harness::{compare_configurations, print_results, RetrievalTestCase};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n");
    println!("🐠══════════════════════════════════════════════════════════🐠");
    println!("   COMPREHENSIVE BENCHMARK - 100+ MEMORIES");
    println!("🐠══════════════════════════════════════════════════════════🐠\n");
    
    let start_time = Instant::now();
    
    // Initialize cortex with absolute path
    let data_dir = std::env::current_dir()?.join("comprehensive_benchmark_data");
    println!("Using data directory: {:?}", data_dir);
    
    // Clear any existing data to avoid lock issues
    if data_dir.exists() {
        println!("Clearing existing benchmark data...");
        std::fs::remove_dir_all(&data_dir)?;
    }
    std::fs::create_dir_all(&data_dir)?;
    
    let cortex = MemoryCortex::new(&data_dir).await?;
    
    // Create large test dataset
    let memories = create_large_test_dataset();
    
    println!("Loading {} test memories into cortex...", memories.len());
    let load_start = Instant::now();
    
    for (i, mem) in memories.iter().enumerate() {
        cortex.remember(mem).await?;
        if (i + 1) % 20 == 0 {
            println!("  ✓ Loaded {}/{} memories ({:.1}%)", 
                i + 1, 
                memories.len(),
                ((i + 1) as f64 / memories.len() as f64) * 100.0
            );
        }
    }
    
    let load_time = load_start.elapsed().as_secs_f64();
    println!("  ✓ All {} memories loaded in {:.2}s\n", memories.len(), load_time);
    
    // Rebuild search index
    println!("Rebuilding BM25 search index...");
    let index_start = Instant::now();
    let indexed = cortex.rebuild_search_index().await?;
    println!("  ✓ Indexed {} memories in {:.2}s\n", indexed, index_start.elapsed().as_secs_f64());
    
    // Rebuild corpus stats
    println!("Rebuilding corpus statistics...");
    cortex.rebuild_corpus_stats().await?;
    println!("  ✓ Corpus stats updated\n");
    
    // Create comprehensive test cases
    let test_cases = create_comprehensive_test_cases();
    println!("Test dataset: {} queries to evaluate\n", test_cases.len());
    
    // Run benchmarks
    println!("Running comprehensive benchmarks...");
    println!("This may take a moment for {} queries...\n", test_cases.len());
    
    let benchmark_start = Instant::now();
    let results = compare_configurations(&cortex, &test_cases).await?;
    let benchmark_time = benchmark_start.elapsed().as_secs_f64();
    
    // Print results
    print_results(&results);
    
    // Print performance metrics
    let total_time = start_time.elapsed().as_secs_f64();
    println!("\n📊 PERFORMANCE METRICS:");
    println!("   • Total setup time: {:.2}s", total_time - benchmark_time);
    println!("   • Benchmark time: {:.2}s", benchmark_time);
    println!("   • Average query time: {:.2}ms", (benchmark_time * 1000.0) / test_cases.len() as f64);
    println!("   • Memories loaded: {}", memories.len());
    println!("   • Queries tested: {}\n", test_cases.len());
    
    // Print sample queries with details
    println!("📋 SAMPLE QUERY ANALYSIS:\n");
    if let Some(hybrid) = results.iter().find(|r| r.name.contains("Hybrid")) {
        // Show best performing queries
        let mut sorted_queries: Vec<_> = hybrid.query_results.iter().enumerate().collect();
        sorted_queries.sort_by(|a, b| b.1.precision.partial_cmp(&a.1.precision).unwrap());
        
        println!("Top 5 Best Performing Queries:");
        for (i, (idx, qr)) in sorted_queries.iter().take(5).enumerate() {
            println!("  {}. \"{}\" - Precision: {:.1}%, Recall: {:.1}%", 
                i + 1, 
                qr.query, 
                qr.precision * 100.0,
                qr.recall * 100.0
            );
        }
        
        println!("\nBottom 5 Queries (Improvement Opportunities):");
        for (i, (idx, qr)) in sorted_queries.iter().rev().take(5).enumerate() {
            println!("  {}. \"{}\" - Precision: {:.1}%, Recall: {:.1}%", 
                i + 1, 
                qr.query, 
                qr.precision * 100.0,
                qr.recall * 100.0
            );
        }
    }
    
    println!("\n✅ Comprehensive benchmark complete!");
    println!("   Total time: {:.2}s\n", total_time);
    
    Ok(())
}

/// Create a comprehensive dataset of 100+ memories
fn create_large_test_dataset() -> Vec<Memory> {
    let mut memories = Vec::new();
    
    // Section 1: Identity & Personal Info (10 memories)
    memories.extend(vec![
        Memory::new("User's name is Alex Johnson", MemoryType::Identity),
        Memory::new("User works as a senior software engineer at Google", MemoryType::Identity),
        Memory::new("User lives in San Francisco, California, Mission District", MemoryType::Identity),
        Memory::new("User is 28 years old", MemoryType::Identity),
        Memory::new("User has a golden retriever named Rusty", MemoryType::Identity),
        Memory::new("User speaks English, Spanish, and basic Mandarin", MemoryType::Identity),
        Memory::new("User graduated from Stanford University with CS degree", MemoryType::Identity),
        Memory::new("User is originally from Austin, Texas", MemoryType::Identity),
        Memory::new("User has been programming for 10 years", MemoryType::Identity),
        Memory::new("User's email is alex.johnson@email.com", MemoryType::Identity),
    ]);
    
    // Section 2: Work Preferences (15 memories)
    memories.extend(vec![
        Memory::new("User prefers working remotely 3 days a week", MemoryType::Preference)
            .with_importance(0.8),
        Memory::new("User likes deep work sessions in the morning", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers async communication over meetings", MemoryType::Preference)
            .with_importance(0.8),
        Memory::new("User likes VS Code with vim keybindings", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User prefers dark mode in all applications", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User dislikes video calls before 10am", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers written documentation over verbal explanations", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User likes code reviews to be thorough and educational", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers pair programming for complex tasks", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User likes to take breaks every 90 minutes", MemoryType::Preference)
            .with_importance(0.5),
        Memory::new("User prefers standups to be under 15 minutes", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User likes to prototype quickly before planning", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers functional programming patterns", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User dislikes micromanagement", MemoryType::Preference)
            .with_importance(0.8),
        Memory::new("User likes to have autonomy over their work schedule", MemoryType::Preference)
            .with_importance(0.8),
    ]);
    
    // Section 3: Technical Skills & Knowledge (20 memories)
    memories.extend(vec![
        Memory::new("User is expert in Rust and systems programming", MemoryType::Fact)
            .with_importance(0.9),
        Memory::new("User knows Python for data science and ML", MemoryType::Fact)
            .with_importance(0.8),
        Memory::new("User is proficient in TypeScript and React", MemoryType::Fact)
            .with_importance(0.8),
        Memory::new("User knows SQL and database design", MemoryType::Fact)
            .with_importance(0.8),
        Memory::new("User is familiar with Docker and Kubernetes", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User knows AWS services: EC2, S3, Lambda, RDS", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User is learning Go for microservices", MemoryType::Fact)
            .with_importance(0.6),
        Memory::new("User understands distributed systems concepts", MemoryType::Fact)
            .with_importance(0.8),
        Memory::new("User knows CI/CD pipelines and GitHub Actions", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User is familiar with GraphQL and REST APIs", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User knows machine learning basics with PyTorch", MemoryType::Fact)
            .with_importance(0.6),
        Memory::new("User understands blockchain and smart contracts", MemoryType::Fact)
            .with_importance(0.5),
        Memory::new("User is learning WebAssembly for performance-critical code", MemoryType::Fact)
            .with_importance(0.6),
        Memory::new("User knows PostgreSQL, MongoDB, and Redis", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User is familiar with gRPC and Protocol Buffers", MemoryType::Fact)
            .with_importance(0.6),
        Memory::new("User understands OAuth2 and JWT authentication", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User knows testing methodologies: TDD, BDD, integration testing", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User is familiar with monitoring tools: Prometheus, Grafana", MemoryType::Fact)
            .with_importance(0.6),
        Memory::new("User understands CAP theorem and database tradeoffs", MemoryType::Fact)
            .with_importance(0.7),
        Memory::new("User knows how to optimize SQL queries for performance", MemoryType::Fact)
            .with_importance(0.7),
    ]);
    
    // Section 4: Goals & Objectives (15 memories)
    memories.extend(vec![
        Memory::new("Goal: Learn Rust programming language to expert level", MemoryType::Goal)
            .with_importance(0.95),
        Memory::new("Goal: Build and launch a side project within 6 months", MemoryType::Goal)
            .with_importance(0.9),
        Memory::new("Goal: Get AWS Solutions Architect certification", MemoryType::Goal)
            .with_importance(0.85),
        Memory::new("Goal: Contribute to open source projects monthly", MemoryType::Goal)
            .with_importance(0.8),
        Memory::new("Goal: Write technical blog posts weekly", MemoryType::Goal)
            .with_importance(0.75),
        Memory::new("Goal: Exercise 4 times per week minimum", MemoryType::Goal)
            .with_importance(0.8),
        Memory::new("Goal: Read 24 technical books this year", MemoryType::Goal)
            .with_importance(0.75),
        Memory::new("Goal: Speak at 3 tech conferences", MemoryType::Goal)
            .with_importance(0.85),
        Memory::new("Goal: Mentor junior developers", MemoryType::Goal)
            .with_importance(0.8),
        Memory::new("Goal: Build a passive income stream", MemoryType::Goal)
            .with_importance(0.85),
        Memory::new("Goal: Learn system design to senior level", MemoryType::Goal)
            .with_importance(0.9),
        Memory::new("Goal: Complete a machine learning course", MemoryType::Goal)
            .with_importance(0.7),
        Memory::new("Goal: Network with 50+ developers in the community", MemoryType::Goal)
            .with_importance(0.75),
        Memory::new("Goal: Achieve work-life balance with 40 hour weeks", MemoryType::Goal)
            .with_importance(0.85),
        Memory::new("Goal: Save 6 months of emergency funds", MemoryType::Goal)
            .with_importance(0.8),
    ]);
    
    // Section 5: Decisions (15 memories)
    memories.extend(vec![
        Memory::new("Decision: Use SQLite for local development database", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Switch to MacBook Pro M3 for development", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Adopt Docker for all development environments", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Use Figma for all design work", MemoryType::Decision)
            .with_importance(0.75),
        Memory::new("Decision: Cancel Netflix subscription to focus on learning", MemoryType::Decision)
            .with_importance(0.7),
        Memory::new("Decision: Use GitHub Actions for CI/CD", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Migrate from JavaScript to TypeScript", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Use PostgreSQL for production database", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Adopt Agile methodology with 2-week sprints", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Use Terraform for infrastructure as code", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Implement microservices architecture for new project", MemoryType::Decision)
            .with_importance(0.85),
        Memory::new("Decision: Use Redis for caching layer", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Adopt trunk-based development workflow", MemoryType::Decision)
            .with_importance(0.75),
        Memory::new("Decision: Use React with Next.js for frontend", MemoryType::Decision)
            .with_importance(0.8),
        Memory::new("Decision: Implement feature flags for gradual rollouts", MemoryType::Decision)
            .with_importance(0.8),
    ]);
    
    // Section 6: Personal Preferences & Lifestyle (15 memories)
    memories.extend(vec![
        Memory::new("User likes coffee, especially oat milk lattes", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User enjoys hiking on weekends in nearby trails", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers reading books over watching videos", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User likes Thai food, especially pad thai and green curry", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User enjoys playing guitar in free time", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User prefers minimal UI designs without clutter", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User likes to travel and explore new cities", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User prefers async communication like email over chat", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User dislikes notification sounds and keeps phone on silent", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User likes to meal prep on Sundays", MemoryType::Preference)
            .with_importance(0.5),
        Memory::new("User prefers electric vehicles and sustainability", MemoryType::Preference)
            .with_importance(0.7),
        Memory::new("User enjoys podcasts about technology and startups", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User prefers standing desk for better posture", MemoryType::Preference)
            .with_importance(0.6),
        Memory::new("User likes ambient music while coding", MemoryType::Preference)
            .with_importance(0.5),
        Memory::new("User prefers to wake up at 7am and sleep by 11pm", MemoryType::Preference)
            .with_importance(0.6),
    ]);
    
    // Section 7: Recent Events (10 memories)
    memories.extend(vec![
        Memory::new("Last week: Presented at Rust meetup about memory safety", MemoryType::Event),
        Memory::new("Yesterday: Had coffee with mentor to discuss career growth", MemoryType::Event),
        Memory::new("Today: Deployed v2.0 of the project to production", MemoryType::Event),
        Memory::new("Last month: Moved to new apartment in Mission District", MemoryType::Event),
        Memory::new("Two weeks ago: Adopted a golden retriever puppy named Rusty", MemoryType::Event),
        Memory::new("Last week: Completed AWS certification exam", MemoryType::Event),
        Memory::new("Yesterday: Pair programmed with colleague on new feature", MemoryType::Event),
        Memory::new("Today: Published technical blog post on Rust async", MemoryType::Event),
        Memory::new("Last weekend: Attended tech conference in San Jose", MemoryType::Event),
        Memory::new("Two days ago: Fixed critical production bug", MemoryType::Event),
    ]);
    
    memories
}

/// Create 30+ comprehensive test queries
fn create_comprehensive_test_cases() -> Vec<RetrievalTestCase> {
    vec![
        // Identity queries
        RetrievalTestCase {
            query: "what is the user's name".to_string(),
            expected_keywords: vec!["name".to_string(), "alex".to_string()],
            description: "Find user identity".to_string(),
        },
        RetrievalTestCase {
            query: "where does user work".to_string(),
            expected_keywords: vec!["google".to_string(), "software engineer".to_string(), "work".to_string()],
            description: "Find workplace".to_string(),
        },
        RetrievalTestCase {
            query: "user location".to_string(),
            expected_keywords: vec!["san francisco".to_string(), "mission district".to_string(), "lives".to_string()],
            description: "Find location".to_string(),
        },
        RetrievalTestCase {
            query: "how old is user".to_string(),
            expected_keywords: vec!["28".to_string(), "years old".to_string(), "age".to_string()],
            description: "Find age".to_string(),
        },
        
        // Work preferences
        RetrievalTestCase {
            query: "user work preferences".to_string(),
            expected_keywords: vec!["remote".to_string(), "deep work".to_string(), "async".to_string(), "meetings".to_string()],
            description: "Find work preferences".to_string(),
        },
        RetrievalTestCase {
            query: "how does user like to work".to_string(),
            expected_keywords: vec!["remote".to_string(), "async".to_string(), "autonomy".to_string()],
            description: "Find work style".to_string(),
        },
        RetrievalTestCase {
            query: "user editor preferences".to_string(),
            expected_keywords: vec!["vs code".to_string(), "vim".to_string(), "editor".to_string()],
            description: "Find editor preferences".to_string(),
        },
        RetrievalTestCase {
            query: "user dislikes about work".to_string(),
            expected_keywords: vec!["micromanagement".to_string(), "video calls".to_string(), "dislikes".to_string()],
            description: "Find work dislikes".to_string(),
        },
        
        // Technical skills
        RetrievalTestCase {
            query: "what programming languages does user know".to_string(),
            expected_keywords: vec!["rust".to_string(), "python".to_string(), "typescript".to_string(), "go".to_string()],
            description: "Find programming skills".to_string(),
        },
        RetrievalTestCase {
            query: "user database skills".to_string(),
            expected_keywords: vec!["postgresql".to_string(), "mongodb".to_string(), "redis".to_string(), "sql".to_string()],
            description: "Find database knowledge".to_string(),
        },
        RetrievalTestCase {
            query: "user cloud experience".to_string(),
            expected_keywords: vec!["aws".to_string(), "ec2".to_string(), "s3".to_string(), "lambda".to_string()],
            description: "Find cloud skills".to_string(),
        },
        RetrievalTestCase {
            query: "user devops knowledge".to_string(),
            expected_keywords: vec!["docker".to_string(), "kubernetes".to_string(), "ci/cd".to_string(), "terraform".to_string()],
            description: "Find devops skills".to_string(),
        },
        RetrievalTestCase {
            query: "user frontend skills".to_string(),
            expected_keywords: vec!["react".to_string(), "typescript".to_string(), "next.js".to_string(), "javascript".to_string()],
            description: "Find frontend skills".to_string(),
        },
        
        // Goals
        RetrievalTestCase {
            query: "what is user learning".to_string(),
            expected_keywords: vec!["goal".to_string(), "learn".to_string(), "rust".to_string(), "go".to_string(), "webassembly".to_string()],
            description: "Find learning goals".to_string(),
        },
        RetrievalTestCase {
            query: "user goals for this year".to_string(),
            expected_keywords: vec!["goal".to_string(), "certification".to_string(), "launch".to_string(), "conferences".to_string()],
            description: "Find yearly goals".to_string(),
        },
        RetrievalTestCase {
            query: "user career goals".to_string(),
            expected_keywords: vec!["senior".to_string(), "system design".to_string(), "mentor".to_string(), "speak".to_string()],
            description: "Find career goals".to_string(),
        },
        RetrievalTestCase {
            query: "user health goals".to_string(),
            expected_keywords: vec!["exercise".to_string(), "work-life".to_string(), "sleep".to_string()],
            description: "Find health goals".to_string(),
        },
        
        // Decisions
        RetrievalTestCase {
            query: "technology choices".to_string(),
            expected_keywords: vec!["sqlite".to_string(), "postgresql".to_string(), "docker".to_string(), "react".to_string()],
            description: "Find tech decisions".to_string(),
        },
        RetrievalTestCase {
            query: "user decided to use".to_string(),
            expected_keywords: vec!["decision".to_string(), "use".to_string(), "adopt".to_string()],
            description: "Find recent decisions".to_string(),
        },
        RetrievalTestCase {
            query: "database decisions".to_string(),
            expected_keywords: vec!["sqlite".to_string(), "postgresql".to_string(), "redis".to_string(), "database".to_string()],
            description: "Find database decisions".to_string(),
        },
        RetrievalTestCase {
            query: "architecture decisions".to_string(),
            expected_keywords: vec!["microservices".to_string(), "terraform".to_string(), "infrastructure".to_string()],
            description: "Find architecture decisions".to_string(),
        },
        
        // Personal preferences
        RetrievalTestCase {
            query: "what does user like".to_string(),
            expected_keywords: vec!["like".to_string(), "enjoy".to_string(), "prefer".to_string(), "coffee".to_string(), "hiking".to_string()],
            description: "Find likes".to_string(),
        },
        RetrievalTestCase {
            query: "user hobbies".to_string(),
            expected_keywords: vec!["hiking".to_string(), "guitar".to_string(), "reading".to_string(), "travel".to_string()],
            description: "Find hobbies".to_string(),
        },
        RetrievalTestCase {
            query: "user lifestyle".to_string(),
            expected_keywords: vec!["coffee".to_string(), "meal prep".to_string(), "wake up".to_string(), "exercise".to_string()],
            description: "Find lifestyle preferences".to_string(),
        },
        RetrievalTestCase {
            query: "user food preferences".to_string(),
            expected_keywords: vec!["thai".to_string(), "pad thai".to_string(), "food".to_string()],
            description: "Find food preferences".to_string(),
        },
        
        // Communication style
        RetrievalTestCase {
            query: "how to contact user".to_string(),
            expected_keywords: vec!["email".to_string(), "async".to_string(), "slack".to_string()],
            description: "Find communication preferences".to_string(),
        },
        RetrievalTestCase {
            query: "user communication preferences".to_string(),
            expected_keywords: vec!["async".to_string(), "email".to_string(), "communication".to_string()],
            description: "Find communication style".to_string(),
        },
        
        // Recent events
        RetrievalTestCase {
            query: "what happened recently".to_string(),
            expected_keywords: vec!["last week".to_string(), "yesterday".to_string(), "today".to_string(), "presented".to_string()],
            description: "Find recent events".to_string(),
        },
        RetrievalTestCase {
            query: "recent achievements".to_string(),
            expected_keywords: vec!["certification".to_string(), "deployed".to_string(), "presented".to_string(), "published".to_string()],
            description: "Find recent achievements".to_string(),
        },
        
        // Complex queries
        RetrievalTestCase {
            query: "user skills related to rust".to_string(),
            expected_keywords: vec!["rust".to_string(), "systems programming".to_string(), "memory safety".to_string()],
            description: "Find Rust-related skills".to_string(),
        },
        RetrievalTestCase {
            query: "user preferences for development tools".to_string(),
            expected_keywords: vec!["vs code".to_string(), "docker".to_string(), "macbook".to_string(), "tools".to_string()],
            description: "Find dev tool preferences".to_string(),
        },
        RetrievalTestCase {
            query: "user experience with databases".to_string(),
            expected_keywords: vec!["postgresql".to_string(), "mongodb".to_string(), "redis".to_string(), "sqlite".to_string()],
            description: "Find database experience".to_string(),
        },
    ]
}
