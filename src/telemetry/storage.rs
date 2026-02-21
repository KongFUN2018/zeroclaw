//! Telemetry storage using SQLite
//!
//! Stores usage metrics locally with no cloud transmission.

use crate::telemetry::TelemetryConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use tracing::debug;

/// Telemetry event types
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    /// Skill was invoked
    SkillUsage,
    /// Tool was called
    ToolUsage,
    /// MCP server was used
    McpUsage,
    /// Agent made a routing decision
    AgentDecision,
}

impl EventType {
    fn as_str(&self) -> &'static str {
        match self {
            EventType::SkillUsage => "skill_usage",
            EventType::ToolUsage => "tool_usage",
            EventType::McpUsage => "mcp_usage",
            EventType::AgentDecision => "agent_decision",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "skill_usage" => Some(EventType::SkillUsage),
            "tool_usage" => Some(EventType::ToolUsage),
            "mcp_usage" => Some(EventType::McpUsage),
            "agent_decision" => Some(EventType::AgentDecision),
            _ => None,
        }
    }
}

/// A telemetry event
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    /// Event type
    pub event_type: EventType,
    /// Name of the component (skill/tool/mcp name)
    pub component_name: String,
    /// Optional metadata (JSON string)
    pub metadata: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Success flag (true = success, false = failure/error)
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
}

/// Telemetry storage backend
pub struct TelemetryStorage {
    conn: Arc<Mutex<Connection>>,
}

impl TelemetryStorage {
    /// Open or create a telemetry database
    pub fn open(config: &TelemetryConfig) -> Result<Self> {
        let conn = Connection::open(&config.db_path)?;

        // Enable WAL mode for better concurrency (returns result, so we use query_row)
        let _wal_mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;

        // Create tables
        Self::create_tables(&conn)?;

        debug!("Telemetry storage opened at {}", config.db_path.display());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create database tables
    fn create_tables(conn: &Connection) -> Result<()> {
        // Main events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                component_name TEXT NOT NULL,
                metadata TEXT,
                timestamp TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 1,
                duration_ms INTEGER
            )",
            [],
        )?;

        // Create indexes for common queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_component ON events(component_name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Record a telemetry event
    pub fn record_event(&self, event: &TelemetryEvent) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute(
            "INSERT INTO events (event_type, component_name, metadata, timestamp, success, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.event_type.as_str(),
                event.component_name,
                event.metadata,
                event.timestamp.to_rfc3339(),
                if event.success { 1 } else { 0 },
                event.duration_ms,
            ],
        )?;

        debug!("Recorded telemetry event: {}", event.event_type.as_str());
        Ok(())
    }

    /// Get usage statistics for a component
    pub fn get_component_stats(
        &self,
        component_name: &str,
        event_type: EventType,
    ) -> Result<ComponentStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;

        // Get total calls first
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE component_name = ?1 AND event_type = ?2",
                params![component_name, event_type.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Get successful calls
        let successful: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE component_name = ?1 AND event_type = ?2 AND success = 1",
            params![component_name, event_type.as_str()],
            |row| row.get(0),
        ).unwrap_or(0);

        // Get average duration
        let avg_duration: f64 = conn
            .query_row(
                "SELECT AVG(duration_ms) FROM events WHERE component_name = ?1 AND event_type = ?2",
                params![component_name, event_type.as_str()],
                |row| row.get::<_, Option<f64>>(0).map(|v| v.unwrap_or(0.0)),
            )
            .unwrap_or(0.0);

        Ok(ComponentStats {
            total_calls: total,
            successful_calls: successful,
            avg_duration_ms: avg_duration,
        })
    }

    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Result<Vec<TelemetryEvent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT event_type, component_name, metadata, timestamp, success, duration_ms
             FROM events
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let mut events = Vec::new();
        let mut rows = stmt.query(params![limit as i64])?;

        while let Some(row) = rows.next()? {
            let event_type_str: String = row.get(0)?;
            let event_type =
                EventType::from_str(&event_type_str).unwrap_or(EventType::AgentDecision);

            let timestamp_str: String = row.get(3)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            events.push(TelemetryEvent {
                event_type,
                component_name: row.get(1)?,
                metadata: row.get(2)?,
                timestamp,
                success: row.get::<_, i32>(4).unwrap_or(1) == 1,
                duration_ms: row.get(5)?,
            });
        }

        Ok(events)
    }

    /// Get all component names for an event type
    pub fn get_component_names(&self, event_type: EventType) -> Result<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT component_name
             FROM events
             WHERE event_type = ?1
             ORDER BY component_name",
        )?;

        let mut names = Vec::new();
        let mut rows = stmt.query(params![event_type.as_str()])?;

        while let Some(row) = rows.next()? {
            names.push(row.get(0)?);
        }

        Ok(names)
    }
}

/// Usage statistics for a component
#[derive(Debug, Clone, Default)]
pub struct ComponentStats {
    /// Total number of calls
    pub total_calls: i64,
    /// Number of successful calls
    pub successful_calls: i64,
    /// Average duration in milliseconds
    pub avg_duration_ms: f64,
}

impl ComponentStats {
    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 1.0;
        }
        self.successful_calls as f64 / self.total_calls as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_storage() -> TelemetryStorage {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = TelemetryConfig {
            enabled: true,
            db_path,
        };
        TelemetryStorage::open(&config).unwrap()
    }

    #[test]
    fn test_storage_open() {
        let storage = test_storage();
        assert!(storage.conn.lock().is_ok());
    }

    #[test]
    fn test_record_event() {
        let storage = test_storage();

        let event = TelemetryEvent {
            event_type: EventType::SkillUsage,
            component_name: "test_skill".to_string(),
            metadata: None,
            timestamp: Utc::now(),
            success: true,
            duration_ms: Some(100),
        };

        storage.record_event(&event).unwrap();
    }

    #[test]
    fn test_get_component_stats() {
        let storage = test_storage();

        // Record some events
        for i in 0..5 {
            let event = TelemetryEvent {
                event_type: EventType::ToolUsage,
                component_name: "test_tool".to_string(),
                metadata: None,
                timestamp: Utc::now(),
                success: i < 4, // One failure
                duration_ms: Some(50 + i * 10),
            };
            storage.record_event(&event).unwrap();
        }

        let stats = storage
            .get_component_stats("test_tool", EventType::ToolUsage)
            .unwrap();
        assert_eq!(stats.total_calls, 5);
        assert_eq!(stats.successful_calls, 4);
        assert_eq!(stats.success_rate(), 0.8);
    }

    #[test]
    fn test_get_recent_events() {
        let storage = test_storage();

        // Record multiple events
        for i in 0..3 {
            let event = TelemetryEvent {
                event_type: EventType::SkillUsage,
                component_name: format!("skill_{}", i),
                metadata: None,
                timestamp: Utc::now(),
                success: true,
                duration_ms: None,
            };
            storage.record_event(&event).unwrap();
        }

        let events = storage.get_recent_events(2).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_get_component_names() {
        let storage = test_storage();

        // Record events for different components
        for name in &["skill_a", "skill_b", "skill_a"] {
            let event = TelemetryEvent {
                event_type: EventType::SkillUsage,
                component_name: name.to_string(),
                metadata: None,
                timestamp: Utc::now(),
                success: true,
                duration_ms: None,
            };
            storage.record_event(&event).unwrap();
        }

        let names = storage.get_component_names(EventType::SkillUsage).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"skill_a".to_string()));
        assert!(names.contains(&"skill_b".to_string()));
    }

    #[test]
    fn test_event_type_conversion() {
        assert_eq!(EventType::SkillUsage.as_str(), "skill_usage");
        assert_eq!(EventType::ToolUsage.as_str(), "tool_usage");
        assert_eq!(EventType::McpUsage.as_str(), "mcp_usage");
        assert_eq!(EventType::AgentDecision.as_str(), "agent_decision");

        assert_eq!(
            EventType::from_str("skill_usage"),
            Some(EventType::SkillUsage)
        );
        assert_eq!(EventType::from_str("unknown"), None);
    }

    #[test]
    fn test_component_stats_default() {
        let stats = ComponentStats::default();
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.successful_calls, 0);
        assert_eq!(stats.success_rate(), 1.0); // No calls = perfect success
    }

    #[test]
    fn test_multiple_event_types() {
        let storage = test_storage();

        // Record different event types
        let skill_event = TelemetryEvent {
            event_type: EventType::SkillUsage,
            component_name: "test".to_string(),
            metadata: None,
            timestamp: Utc::now(),
            success: true,
            duration_ms: None,
        };

        let tool_event = TelemetryEvent {
            event_type: EventType::ToolUsage,
            component_name: "test".to_string(),
            metadata: None,
            timestamp: Utc::now(),
            success: true,
            duration_ms: None,
        };

        storage.record_event(&skill_event).unwrap();
        storage.record_event(&tool_event).unwrap();

        // Stats should be separate by type
        let skill_stats = storage
            .get_component_stats("test", EventType::SkillUsage)
            .unwrap();
        let tool_stats = storage
            .get_component_stats("test", EventType::ToolUsage)
            .unwrap();

        assert_eq!(skill_stats.total_calls, 1);
        assert_eq!(tool_stats.total_calls, 1);
    }
}
