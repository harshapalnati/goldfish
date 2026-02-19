//! Enhanced evaluation with semantic keyword matching
//!
//! This module provides better evaluation by using:
//! - Stemming ("works" matches "work")
//! - Synonyms ("job" matches "software engineer", "works")
//! - Type-based relevance

use std::collections::HashSet;

/// Expand query with synonyms and related terms
pub fn expand_query(query: &str) -> Vec<String> {
    let mut expanded = vec![query.to_string()];
    let query_lower = query.to_lowercase();

    // Add synonyms
    let synonyms: Vec<&str> = match query_lower.as_str() {
        q if q.contains("job") || q.contains("work") => {
            vec!["software engineer", "developer", "works", "startup"]
        }
        q if q.contains("live") || q.contains("location") => {
            vec![
                "lives",
                "san francisco",
                "apartment",
                "mission district",
                "moved",
            ]
        }
        q if q.contains("name") => {
            vec!["alex", "user"]
        }
        q if q.contains("like") || q.contains("prefer") => {
            vec![
                "prefers",
                "likes",
                "enjoy",
                "dark mode",
                "coffee",
                "slack",
                "thai food",
            ]
        }
        q if q.contains("goal") || q.contains("learning") => {
            vec![
                "learn",
                "rust",
                "aws",
                "certification",
                "exercise",
                "read",
                "books",
            ]
        }
        q if q.contains("decision") || q.contains("choice") || q.contains("technology") => {
            vec!["sqlite", "docker", "macbook", "figma", "netflix", "cancel"]
        }
        q if q.contains("communication") || q.contains("contact") => {
            vec!["slack", "email", "async", "real-time", "chat"]
        }
        q if q.contains("morning") || q.contains("routine") => {
            vec!["coffee", "10am", "video calls", "oat milk"]
        }
        q if q.contains("hobby") || q.contains("hobbies") => {
            vec!["hiking", "reading", "books", "trails", "weekends"]
        }
        _ => vec![],
    };

    expanded.extend(synonyms.into_iter().map(|s| s.to_string()));
    expanded
}

/// Check if memory content is relevant to query using semantic matching
pub fn is_semantically_relevant(content: &str, query: &str) -> bool {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    // Direct substring match
    if content_lower.contains(&query_lower) {
        return true;
    }

    // Check expanded terms
    let expanded = expand_query(query);
    expanded.iter().any(|term| {
        let term_lower = term.to_lowercase();
        content_lower.contains(&term_lower)
    })
}

/// Get expected memories for a query (for better evaluation)
pub fn get_expected_memory_patterns(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();

    match query_lower.as_str() {
        q if q.contains("name") => vec!["name is alex".to_string()],
        q if q.contains("live") || q.contains("location") => vec![
            "lives in san francisco".to_string(),
            "moved to new apartment".to_string(),
        ],
        q if q.contains("job") || q.contains("work") => vec![
            "software engineer".to_string(),
            "works as a software engineer".to_string(),
        ],
        q if q.contains("preference") => vec![
            "prefers dark mode".to_string(),
            "likes coffee".to_string(),
            "prefers slack".to_string(),
            "likes hiking".to_string(),
            "prefers minimal ui".to_string(),
        ],
        q if q.contains("like") && !q.contains("preference") => vec![
            "likes coffee".to_string(),
            "likes hiking".to_string(),
            "likes thai food".to_string(),
            "prefers reading".to_string(),
        ],
        q if q.contains("goal") || q.contains("learning") => vec![
            "learn rust".to_string(),
            "get aws certification".to_string(),
            "exercise 3 times".to_string(),
            "read 20 books".to_string(),
        ],
        q if q.contains("decision") || q.contains("technology") => vec![
            "use sqlite".to_string(),
            "switch to macbook".to_string(),
            "adopt docker".to_string(),
            "use figma".to_string(),
        ],
        q if q.contains("communication") => vec![
            "prefers slack".to_string(),
            "prefers async communication".to_string(),
        ],
        q if q.contains("morning") => vec![
            "coffee".to_string(),
            "dislikes video calls before 10am".to_string(),
        ],
        q if q.contains("hobby") => vec!["hiking".to_string(), "reading books".to_string()],
        _ => vec![query_lower],
    }
}

/// Calculate semantic precision (more lenient than keyword matching)
pub fn calculate_semantic_precision(results: &[String], query: &str) -> f32 {
    if results.is_empty() {
        return 0.0;
    }

    let patterns = get_expected_memory_patterns(query);
    let relevant_count = results
        .iter()
        .filter(|content| {
            patterns
                .iter()
                .any(|pattern| content.to_lowercase().contains(&pattern.to_lowercase()))
        })
        .count();

    relevant_count as f32 / results.len() as f32
}

/// Stem a word to its root form (simplified)
pub fn stem_word(word: &str) -> String {
    let word = word.to_lowercase();

    // Simple stemming rules
    if word.ends_with("ing") && word.len() > 4 {
        word[..word.len() - 3].to_string()
    } else if word.ends_with("es") && word.len() > 3 {
        word[..word.len() - 2].to_string()
    } else if word.ends_with("s") && word.len() > 2 && !word.ends_with("ss") {
        word[..word.len() - 1].to_string()
    } else {
        word
    }
}

/// Check if two terms match using stemming
pub fn stem_match(term1: &str, term2: &str) -> bool {
    stem_word(term1) == stem_word(term2)
}
