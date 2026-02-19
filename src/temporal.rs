//! Temporal memory queries
//!
//! This module enables time-based memory retrieval, allowing queries like:
//! - "What happened yesterday?"
//! - "What did we discuss last Tuesday?"
//! - "Memories from the past week"
//!
//! Based on research in episodic memory and temporal cognition.

use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Temporal query specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalQuery {
    /// Start time (inclusive)
    pub start: Option<DateTime<Utc>>,

    /// End time (inclusive)
    pub end: Option<DateTime<Utc>>,

    /// Query mode
    pub mode: TemporalMode,

    /// Time range preset
    pub preset: Option<TemporalPreset>,
}

impl TemporalQuery {
    /// Create a new temporal query
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            mode: TemporalMode::Created,
            preset: None,
        }
    }

    /// Query by creation time
    pub fn created() -> Self {
        Self {
            start: None,
            end: None,
            mode: TemporalMode::Created,
            preset: None,
        }
    }

    /// Query by last access time
    pub fn accessed() -> Self {
        Self {
            start: None,
            end: None,
            mode: TemporalMode::LastAccessed,
            preset: None,
        }
    }

    /// Query by update time
    pub fn updated() -> Self {
        Self {
            start: None,
            end: None,
            mode: TemporalMode::Updated,
            preset: None,
        }
    }

    /// Set explicit time range
    pub fn between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self.end = Some(end);
        self.preset = None;
        self
    }

    /// Set start time only
    pub fn after(mut self, start: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self.end = None;
        self.preset = None;
        self
    }

    /// Set end time only
    pub fn before(mut self, end: DateTime<Utc>) -> Self {
        self.start = None;
        self.end = Some(end);
        self.preset = None;
        self
    }

    /// Use a preset time range
    pub fn preset(mut self, preset: TemporalPreset) -> Self {
        let (start, end) = preset.to_range();
        self.start = Some(start);
        self.end = Some(end);
        self.preset = Some(preset);
        self
    }

    /// Query last N days
    pub fn last_days(n: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(n);
        Self {
            start: Some(start),
            end: Some(end),
            mode: TemporalMode::Created,
            preset: Some(TemporalPreset::Custom(format!("last_{}_days", n))),
        }
    }

    /// Query last N hours
    pub fn last_hours(n: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(n);
        Self {
            start: Some(start),
            end: Some(end),
            mode: TemporalMode::Created,
            preset: Some(TemporalPreset::Custom(format!("last_{}_hours", n))),
        }
    }

    /// Query today
    pub fn today() -> Self {
        Self::new().preset(TemporalPreset::Today)
    }

    /// Query yesterday
    pub fn yesterday() -> Self {
        Self::new().preset(TemporalPreset::Yesterday)
    }

    /// Query this week
    pub fn this_week() -> Self {
        Self::new().preset(TemporalPreset::ThisWeek)
    }

    /// Query last week
    pub fn last_week() -> Self {
        Self::new().preset(TemporalPreset::LastWeek)
    }

    /// Query this month
    pub fn this_month() -> Self {
        Self::new().preset(TemporalPreset::ThisMonth)
    }

    /// Convert to SQL WHERE clause
    pub fn to_sql_filter(&self) -> String {
        let column = match self.mode {
            TemporalMode::Created => "created_at",
            TemporalMode::Updated => "updated_at",
            TemporalMode::LastAccessed => "last_accessed_at",
        };

        match (&self.start, &self.end) {
            (Some(start), Some(end)) => {
                format!("{} BETWEEN '{}' AND '{}'", column, start, end)
            }
            (Some(start), None) => {
                format!("{} >= '{}'", column, start)
            }
            (None, Some(end)) => {
                format!("{} <= '{}'", column, end)
            }
            (None, None) => "1=1".to_string(),
        }
    }
}

impl Default for TemporalQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporal query modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TemporalMode {
    /// Query by creation time
    Created,
    /// Query by last access time
    LastAccessed,
    /// Query by update time
    Updated,
}

impl std::fmt::Display for TemporalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemporalMode::Created => write!(f, "created"),
            TemporalMode::LastAccessed => write!(f, "accessed"),
            TemporalMode::Updated => write!(f, "updated"),
        }
    }
}

/// Preset time ranges for common queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TemporalPreset {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Last7Days,
    Last30Days,
    Last24Hours,
    Custom(String),
}

impl TemporalPreset {
    /// Convert preset to time range (start, end)
    pub fn to_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let today_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        match self {
            TemporalPreset::Today => {
                let tomorrow = today_start + Duration::days(1);
                (today_start, tomorrow)
            }
            TemporalPreset::Yesterday => {
                let yesterday_start = today_start - Duration::days(1);
                (yesterday_start, today_start)
            }
            TemporalPreset::ThisWeek => {
                // Start of week (Monday)
                let days_since_monday = now.weekday().num_days_from_monday() as i64;
                let week_start = today_start - Duration::days(days_since_monday);
                (week_start, now)
            }
            TemporalPreset::LastWeek => {
                let days_since_monday = now.weekday().num_days_from_monday() as i64;
                let this_week_start = today_start - Duration::days(days_since_monday);
                let last_week_start = this_week_start - Duration::days(7);
                (last_week_start, this_week_start)
            }
            TemporalPreset::ThisMonth => {
                let month_start = now
                    .with_day(1)
                    .unwrap()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(Utc)
                    .unwrap();
                (month_start, now)
            }
            TemporalPreset::LastMonth => {
                let this_month_start = now.with_day(1).unwrap();
                let last_month_end = this_month_start;
                let last_month_start = (this_month_start - Duration::days(1)).with_day(1).unwrap();
                let last_month_start = last_month_start
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(Utc)
                    .unwrap();
                (last_month_start, last_month_end)
            }
            TemporalPreset::Last7Days => {
                let start = now - Duration::days(7);
                (start, now)
            }
            TemporalPreset::Last30Days => {
                let start = now - Duration::days(30);
                (start, now)
            }
            TemporalPreset::Last24Hours => {
                let start = now - Duration::hours(24);
                (start, now)
            }
            TemporalPreset::Custom(_) => {
                // For custom, caller should set explicit times
                (now - Duration::days(1), now)
            }
        }
    }
}

impl std::fmt::Display for TemporalPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemporalPreset::Today => write!(f, "today"),
            TemporalPreset::Yesterday => write!(f, "yesterday"),
            TemporalPreset::ThisWeek => write!(f, "this_week"),
            TemporalPreset::LastWeek => write!(f, "last_week"),
            TemporalPreset::ThisMonth => write!(f, "this_month"),
            TemporalPreset::LastMonth => write!(f, "last_month"),
            TemporalPreset::Last7Days => write!(f, "last_7_days"),
            TemporalPreset::Last30Days => write!(f, "last_30_days"),
            TemporalPreset::Last24Hours => write!(f, "last_24_hours"),
            TemporalPreset::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Episode represents a continuous period of memory formation
///
/// Useful for grouping related memories into "episodes" or "sessions"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier
    pub id: String,

    /// Episode name/title
    pub title: String,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: DateTime<Utc>,

    /// Memory IDs in this episode
    pub memory_ids: Vec<String>,

    /// Summary of the episode
    pub summary: Option<String>,

    /// Session ID if applicable
    pub session_id: Option<String>,

    /// Participants (for multi-user scenarios)
    pub participants: Vec<String>,
}

impl Episode {
    /// Get duration of episode
    pub fn duration(&self) -> Duration {
        self.end_time - self.start_time
    }

    /// Check if a timestamp falls within this episode
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start_time && timestamp <= self.end_time
    }
}

/// Result of temporal search
#[derive(Debug, Clone, Serialize)]
pub struct TemporalSearchResult {
    /// The episode or time period
    pub period: String,

    /// Start of period
    pub start: DateTime<Utc>,

    /// End of period
    pub end: DateTime<Utc>,

    /// Number of memories in this period
    pub memory_count: usize,

    /// Sample of memories (if requested)
    pub memories: Vec<crate::types::Memory>,
}

/// Configuration for temporal memory features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    /// Enable automatic episode detection
    pub enable_episode_detection: bool,

    /// Gap between memories to start new episode (in minutes)
    pub episode_gap_minutes: i64,

    /// Minimum episode duration (in minutes)
    pub min_episode_duration: i64,

    /// Enable temporal decay of importance
    pub enable_temporal_decay: bool,

    /// Decay rate per day
    pub temporal_decay_rate: f32,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            enable_episode_detection: true,
            episode_gap_minutes: 30,
            min_episode_duration: 5,
            enable_temporal_decay: true,
            temporal_decay_rate: 0.02,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_query_today() {
        let query = TemporalQuery::today();
        assert!(query.start.is_some());
        assert!(query.end.is_some());
        assert_eq!(query.mode, TemporalMode::Created);
    }

    #[test]
    fn test_temporal_query_last_days() {
        let query = TemporalQuery::last_days(7);
        let now = Utc::now();
        let expected_start = now - Duration::days(7);

        assert!(query.start.is_some());
        assert!(query.end.is_some());

        let start = query.start.unwrap();
        let diff = (start - expected_start).num_seconds();
        assert!(diff.abs() < 2); // Within 2 seconds
    }

    #[test]
    fn test_preset_to_range() {
        let preset = TemporalPreset::Last7Days;
        let (start, end) = preset.to_range();

        let diff = (end - start).num_days();
        assert_eq!(diff, 7);
    }

    #[test]
    fn test_sql_filter() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let query = TemporalQuery::created().between(start, end);

        let sql = query.to_sql_filter();
        assert!(sql.contains("created_at"));
        assert!(sql.contains("BETWEEN"));
    }
}
