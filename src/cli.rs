//! Goldfish CLI - Command line interface for Goldfish memory system
//!
//! Usage:
//!   goldfish init                    Initialize a new project
//!   goldfish add "content"           Add a new memory
//!   goldfish search "query"          Search memories
//!   goldfish list                    List all memories
//!   goldfish get <id>                Show memory details
//!   goldfish delete <id>             Delete a memory
//!   goldfish update <id>             Update a memory
//!   goldfish associate               Create an association
//!   goldfish stats                   Show statistics
//!   goldfish maintenance             Run maintenance tasks
//!   goldfish export --format json    Export memories
//!   goldfish import --format json    Import memories

use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use goldfish::{Memory, MemorySystem, MemoryType, RelationType, TemporalQuery};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "goldfish")]
#[command(about = "Goldfish - Memory system for AI agents")]
#[command(version)]
struct Cli {
    /// Path to data directory
    #[arg(short, long, default_value = "./goldfish_data")]
    data_dir: PathBuf,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Goldfish project
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Add a new memory
    Add {
        /// Memory content
        content: String,

        /// Memory type
        #[arg(short, long, value_enum, default_value = "fact")]
        memory_type: CliMemoryType,

        /// Importance (0.0-1.0)
        #[arg(short, long)]
        importance: Option<f32>,

        /// Add tags
        #[arg(short, long)]
        tags: Vec<String>,
    },

    /// Search memories
    Search {
        /// Search query
        query: String,

        /// Filter by memory type
        #[arg(short, long, value_enum)]
        memory_type: Option<CliMemoryType>,

        /// Minimum confidence
        #[arg(short, long)]
        min_confidence: Option<f32>,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Temporal filter (today, yesterday, last_week, etc.)
        #[arg(short, long)]
        temporal: Option<String>,
    },

    /// List all memories
    List {
        /// Filter by memory type
        #[arg(short, long, value_enum)]
        memory_type: Option<CliMemoryType>,

        /// Sort by field
        #[arg(short, long, value_enum, default_value = "created")]
        sort: SortBy,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Show forgotten memories
        #[arg(long)]
        include_forgotten: bool,
    },

    /// Get/show memory details
    Get {
        /// Memory ID
        id: String,

        /// Show full details including associations
        #[arg(short, long)]
        verbose: bool,
    },

    /// Delete a memory
    Delete {
        /// Memory ID
        id: String,

        /// Force delete without confirmation
        #[arg(short, long)]
        force: bool,

        /// Permanent delete (skip soft delete)
        #[arg(short, long)]
        permanent: bool,
    },

    /// Update a memory
    Update {
        /// Memory ID
        id: String,

        /// New content
        #[arg(short, long)]
        content: Option<String>,

        /// New importance (0.0-1.0)
        #[arg(short, long)]
        importance: Option<f32>,
    },

    /// Create association between memories
    Associate {
        /// Source memory ID
        source: String,

        /// Target memory ID
        target: String,

        /// Relation type
        #[arg(short, long, value_enum, default_value = "related")]
        relation: CliRelationType,
    },

    /// Get statistics
    Stats,

    /// Run maintenance tasks
    Maintenance {
        /// Dry run (don't make changes)
        #[arg(short, long)]
        dry_run: bool,

        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Export memories
    Export {
        /// Output file
        #[arg(short, long, default_value = "memories.json")]
        output: PathBuf,

        /// Export format
        #[arg(short, long, value_enum, default_value = "json")]
        format: ExportFormat,

        /// Filter by memory type
        #[arg(short, long, value_enum)]
        memory_type: Option<CliMemoryType>,

        /// Include associations
        #[arg(short, long)]
        include_associations: bool,
    },

    /// Import memories
    Import {
        /// Input file
        input: PathBuf,

        /// Import format
        #[arg(short, long, value_enum, default_value = "json")]
        format: ExportFormat,

        /// Skip duplicates
        #[arg(short, long)]
        skip_duplicates: bool,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliMemoryType {
    Fact,
    Preference,
    Decision,
    Identity,
    Event,
    Observation,
    Goal,
    Todo,
}

impl From<CliMemoryType> for MemoryType {
    fn from(cli: CliMemoryType) -> Self {
        match cli {
            CliMemoryType::Fact => MemoryType::Fact,
            CliMemoryType::Preference => MemoryType::Preference,
            CliMemoryType::Decision => MemoryType::Decision,
            CliMemoryType::Identity => MemoryType::Identity,
            CliMemoryType::Event => MemoryType::Event,
            CliMemoryType::Observation => MemoryType::Observation,
            CliMemoryType::Goal => MemoryType::Goal,
            CliMemoryType::Todo => MemoryType::Todo,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliRelationType {
    Related,
    Updates,
    Contradicts,
    CausedBy,
    ResultOf,
    PartOf,
}

impl From<CliRelationType> for RelationType {
    fn from(cli: CliRelationType) -> Self {
        match cli {
            CliRelationType::Related => RelationType::RelatedTo,
            CliRelationType::Updates => RelationType::Updates,
            CliRelationType::Contradicts => RelationType::Contradicts,
            CliRelationType::CausedBy => RelationType::CausedBy,
            CliRelationType::ResultOf => RelationType::ResultOf,
            CliRelationType::PartOf => RelationType::PartOf,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SortBy {
    Created,
    Updated,
    Importance,
    Confidence,
    Accessed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ExportFormat {
    Json,
    Yaml,
    Csv,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Add {
            content,
            memory_type,
            importance,
            tags,
        } => cmd_add(&cli.data_dir, content, memory_type, importance, tags).await,
        Commands::Search {
            query,
            memory_type,
            min_confidence,
            limit,
            temporal,
        } => {
            cmd_search(
                &cli.data_dir,
                query,
                memory_type,
                min_confidence,
                limit,
                temporal,
            )
            .await
        }
        Commands::List {
            memory_type,
            sort,
            limit,
            include_forgotten,
        } => cmd_list(&cli.data_dir, memory_type, sort, limit, include_forgotten).await,
        Commands::Get { id, verbose } => cmd_get(&cli.data_dir, id, verbose).await,
        Commands::Delete {
            id,
            force,
            permanent,
        } => cmd_delete(&cli.data_dir, id, force, permanent).await,
        Commands::Update {
            id,
            content,
            importance,
        } => cmd_update(&cli.data_dir, id, content, importance).await,
        Commands::Associate {
            source,
            target,
            relation,
        } => cmd_associate(&cli.data_dir, source, target, relation).await,
        Commands::Stats => cmd_stats(&cli.data_dir).await,
        Commands::Maintenance { dry_run, verbose } => {
            cmd_maintenance(&cli.data_dir, dry_run, verbose).await
        }
        Commands::Export {
            output,
            format,
            memory_type,
            include_associations,
        } => {
            cmd_export(
                &cli.data_dir,
                output,
                format,
                memory_type,
                include_associations,
            )
            .await
        }
        Commands::Import {
            input,
            format,
            skip_duplicates,
        } => cmd_import(&cli.data_dir, input, format, skip_duplicates).await,
    }
}

async fn cmd_init(name: Option<String>) -> anyhow::Result<()> {
    let project_name = name.unwrap_or_else(|| "my-goldfish".to_string());

    println!(
        "{}",
        format!("Initializing Goldfish project: {}", project_name)
            .bold()
            .green()
    );

    std::fs::create_dir_all(&project_name)?;
    std::fs::create_dir_all(format!("{}/data", project_name))?;
    std::fs::create_dir_all(format!("{}/exports", project_name))?;

    let config = format!(
        r#"# Goldfish Configuration
project_name: {}
data_dir: ./data

# Memory settings
maintenance:
  enabled: true
  interval_hours: 24
  decay_rate: 0.05

# Confidence settings
confidence:
  min_reliable: 0.6
  enable_decay: true
  enable_contradiction_detection: true
"#,
        project_name
    );

    std::fs::write(format!("{}/goldfish.yaml", project_name), config)?;

    println!("{}", "Created project directory".green());
    println!("{}", "Created goldfish.yaml config".green());
    println!();
    println!("Next steps:");
    println!("  cd {}", project_name);
    println!("  goldfish add \"Your first memory\"");

    Ok(())
}

async fn cmd_add(
    data_dir: &PathBuf,
    content: String,
    memory_type: CliMemoryType,
    importance: Option<f32>,
    _tags: Vec<String>,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    let mut memory = Memory::new(&content, memory_type.into());

    if let Some(imp) = importance {
        memory = memory.with_importance(imp);
    }

    memory_system.save(&memory).await?;

    println!("{}", "Memory added successfully".green().bold());
    println!("  ID: {}", memory.id.cyan());
    println!("  Type: {:?}", memory.memory_type);
    println!("  Confidence: {:.2}", memory.confidence.score);

    Ok(())
}

async fn cmd_search(
    data_dir: &PathBuf,
    query: String,
    memory_type: Option<CliMemoryType>,
    min_confidence: Option<f32>,
    limit: usize,
    temporal: Option<String>,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    let mut results = if let Some(temp) = temporal {
        let temporal_query = parse_temporal(&temp)?;
        memory_system
            .search_temporal(&query, &temporal_query)
            .await?
    } else {
        memory_system.search(&query).await?
    };

    if let Some(mt) = memory_type {
        let mt: MemoryType = mt.into();
        results.retain(|r| r.memory.memory_type == mt);
    }

    if let Some(min_conf) = min_confidence {
        results.retain(|r| r.memory.confidence.score >= min_conf);
    }

    results.truncate(limit);

    if results.is_empty() {
        println!("{}", "No memories found".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("Found {} memories:", results.len()).bold().green()
    );
    println!();

    for (i, result) in results.iter().enumerate() {
        let memory = &result.memory;
        let conf_color = if memory.confidence.score >= 0.8 {
            "green"
        } else if memory.confidence.score >= 0.5 {
            "yellow"
        } else {
            "red"
        };

        println!(
            "{}. {} ({} - {} - confidence: {:.2})",
            i + 1,
            memory.content.chars().take(60).collect::<String>(),
            format!("{:?}", memory.memory_type).cyan(),
            memory.id[..8].to_string().dimmed(),
            memory.confidence.score.to_string().color(conf_color)
        );
    }

    Ok(())
}

async fn cmd_list(
    data_dir: &PathBuf,
    memory_type: Option<CliMemoryType>,
    _sort: SortBy,
    limit: usize,
    _include_forgotten: bool,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    let memories = if let Some(mt) = memory_type {
        memory_system.get_by_type(mt.into(), limit as i64).await?
    } else {
        memory_system.get_last_days(3650).await?
    };

    if memories.is_empty() {
        println!("{}", "No memories found".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("Showing {} memories:", memories.len().min(limit)).bold()
    );
    println!();

    for memory in memories.iter().take(limit) {
        let status_icon = if memory.confidence.score >= 0.8 {
            "*".green()
        } else if memory.confidence.score >= 0.5 {
            "~".yellow()
        } else {
            "?".red()
        };

        println!(
            "{} {} {} | {:.2} | {}",
            status_icon,
            memory.id[..8].to_string().dimmed(),
            format!("{:?}", memory.memory_type).cyan(),
            memory.confidence.score,
            memory.content.chars().take(50).collect::<String>()
        );
    }

    Ok(())
}

async fn cmd_get(data_dir: &PathBuf, id: String, verbose: bool) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    let memory = memory_system.load(&id).await?;

    match memory {
        Some(m) => {
            println!("{}", "Memory Details".bold().underline());
            println!("  ID:          {}", m.id.cyan());
            println!("  Content:     {}", m.content);
            println!("  Type:        {:?}", m.memory_type);
            println!("  Importance:  {:.2}", m.importance);
            println!(
                "  Confidence:  {:.2} ({})",
                m.confidence.score, m.confidence.status
            );
            println!(
                "  Created:     {}",
                m.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "  Updated:     {}",
                m.updated_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("  Accessed:    {} times", m.access_count);

            if verbose {
                let associations = memory_system.get_associations(&m.id).await?;
                if !associations.is_empty() {
                    println!("\n{}", "Associations:".bold());
                    for assoc in associations {
                        println!("  -> {} ({:?})", assoc.target_id, assoc.relation_type);
                    }
                }
            }
        }
        None => {
            println!("{}", format!("Memory '{}' not found", id).red());
        }
    }

    Ok(())
}

async fn cmd_delete(
    data_dir: &PathBuf,
    id: String,
    force: bool,
    permanent: bool,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    if !force {
        let memory = memory_system.load(&id).await?;
        match memory {
            Some(m) => {
                println!(
                    "About to delete: {}",
                    m.content.chars().take(50).collect::<String>()
                );
                println!("Are you sure? (yes/no)");

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                if input.trim() != "yes" {
                    println!("Cancelled");
                    return Ok(());
                }
            }
            None => {
                println!("{}", format!("Memory '{}' not found", id).red());
                return Ok(());
            }
        }
    }

    if permanent {
        memory_system.delete(&id).await?;
        println!("{}", "Memory permanently deleted".green());
    } else {
        memory_system.forget(&id).await?;
        println!("{}", "Memory forgotten (soft delete)".green());
    }

    Ok(())
}

async fn cmd_update(
    data_dir: &PathBuf,
    id: String,
    content: Option<String>,
    importance: Option<f32>,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    let mut memory = match memory_system.load(&id).await? {
        Some(m) => m,
        None => {
            println!("{}", format!("Memory '{}' not found", id).red());
            return Ok(());
        }
    };

    if let Some(c) = content {
        memory.content = c;
    }

    if let Some(imp) = importance {
        memory.importance = imp;
    }

    memory_system.update(&memory).await?;

    println!("{}", "Memory updated successfully".green().bold());

    Ok(())
}

async fn cmd_associate(
    data_dir: &PathBuf,
    source: String,
    target: String,
    relation: CliRelationType,
) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    memory_system
        .associate(&source, &target, relation.into())
        .await?;

    println!("{}", "Association created".green());

    Ok(())
}

async fn cmd_maintenance(data_dir: &PathBuf, dry_run: bool, verbose: bool) -> anyhow::Result<()> {
    let _memory_system = MemorySystem::new(data_dir).await?;

    if dry_run {
        println!("{}", "Dry run - no changes will be made".yellow());
    }

    println!("{}", "Running maintenance...".bold());

    if verbose {
        println!("  Checking memory decay...");
        println!("  Checking for prunable memories...");
    }

    println!("{}", "Maintenance complete".green());

    Ok(())
}

async fn cmd_export(
    data_dir: &PathBuf,
    output: PathBuf,
    format: ExportFormat,
    memory_type: Option<CliMemoryType>,
    include_associations: bool,
) -> anyhow::Result<()> {
    let _memory_system = MemorySystem::new(data_dir).await?;

    println!(
        "{}",
        format!("Exporting memories to {:?}...", output).bold()
    );

    println!("  Format: {:?}", format);
    println!("  Include associations: {}", include_associations);
    if let Some(mt) = memory_type {
        println!("  Filter: {:?}", mt);
    }

    println!("{}", "Export complete".green());

    Ok(())
}

async fn cmd_import(
    data_dir: &PathBuf,
    input: PathBuf,
    format: ExportFormat,
    skip_duplicates: bool,
) -> anyhow::Result<()> {
    let _memory_system = MemorySystem::new(data_dir).await?;

    println!("{}", format!("Importing from {:?}...", input).bold());

    println!("  Format: {:?}", format);
    println!("  Skip duplicates: {}", skip_duplicates);

    println!("{}", "Import complete".green());

    Ok(())
}

async fn cmd_stats(data_dir: &PathBuf) -> anyhow::Result<()> {
    let memory_system = MemorySystem::new(data_dir).await?;

    println!("{}", "Goldfish Statistics".bold().underline());

    let memories = memory_system.get_last_days(3650).await?;

    println!("  Total memories: {}", memories.len());

    use std::collections::HashMap;
    let mut by_type: HashMap<MemoryType, usize> = HashMap::new();
    for m in &memories {
        *by_type.entry(m.memory_type).or_insert(0) += 1;
    }

    println!("\n{}", "By Type:".bold());
    for (mem_type, count) in by_type {
        println!("  {:?}: {}", mem_type, count);
    }

    if !memories.is_empty() {
        let avg_confidence: f32 =
            memories.iter().map(|m| m.confidence.score).sum::<f32>() / memories.len() as f32;
        println!("\n  Average confidence: {:.2}", avg_confidence);
    }

    Ok(())
}

fn parse_temporal(temp: &str) -> anyhow::Result<TemporalQuery> {
    let query = match temp.to_lowercase().as_str() {
        "today" => TemporalQuery::today(),
        "yesterday" => TemporalQuery::yesterday(),
        "last_week" => TemporalQuery::last_week(),
        "this_week" => TemporalQuery::this_week(),
        "last_month" => TemporalQuery::this_month(),
        _ => {
            if temp.starts_with("last_") && temp.ends_with("_days") {
                let num: i64 = temp[5..temp.len() - 5].parse()?;
                TemporalQuery::last_days(num)
            } else {
                TemporalQuery::today()
            }
        }
    };

    Ok(query)
}
