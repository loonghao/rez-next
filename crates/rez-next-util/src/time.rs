//! Time utility functions

use chrono::{DateTime, Local, Utc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Get the current time as a Unix timestamp (seconds since epoch)
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Get the current time as a DateTime<Utc>
pub fn current_time_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Get the current time as a DateTime<Local>
pub fn current_time_local() -> DateTime<Local> {
    Local::now()
}

/// Format a Unix timestamp as an ISO 8601 string
pub fn format_timestamp_iso(timestamp: u64) -> String {
    let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_default();
    datetime.to_rfc3339()
}

/// Format the current time as an ISO 8601 string
pub fn format_current_time_iso() -> String {
    format_timestamp_iso(current_timestamp())
}

/// Parse a duration string (e.g., "1h", "30m", "45s") to a Duration
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().to_lowercase();
    
    if let Some(num_str) = s.strip_suffix('s') {
        let secs: u64 = num_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid number: {}", num_str))?;
        Ok(Duration::from_secs(secs))
    } else if let Some(num_str) = s.strip_suffix('m') {
        let mins: u64 = num_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid number: {}", num_str))?;
        Ok(Duration::from_secs(mins * 60))
    } else if let Some(num_str) = s.strip_suffix('h') {
        let hours: u64 = num_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid number: {}", num_str))?;
        Ok(Duration::from_secs(hours * 3600))
    } else if let Some(num_str) = s.strip_suffix('d') {
        let days: u64 = num_str
            .trim()
            .parse()
            .map_err(|_| format!("Invalid number: {}", num_str))?;
        Ok(Duration::from_secs(days * 86400))
    } else {
        // Try to parse as pure number (seconds)
        let secs: u64 = s
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(secs))
    }
}

/// Get a human-readable time difference (e.g., "5 minutes ago")
pub fn human_time_diff(timestamp: u64) -> String {
    let now = current_timestamp();
    
    if now < timestamp {
        return "in the future".to_string();
    }
    
    let diff = now - timestamp;
    
    if diff < 60 {
        format!("{} seconds ago", diff)
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else if diff < 604800 {
        format!("{} days ago", diff / 86400)
    } else if diff < 2592000 {
        format!("{} weeks ago", diff / 604800)
    } else if diff < 31536000 {
        format!("{} months ago", diff / 2592000)
    } else {
        format!("{} years ago", diff / 31536000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_format_timestamp_iso() {
        let formatted = format_timestamp_iso(0);
        assert!(formatted.contains("1970"));
    }

    #[test]
    fn test_format_current_time_iso() {
        let formatted = format_current_time_iso();
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
        assert_eq!(parse_duration("45").unwrap(), Duration::from_secs(45));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_human_time_diff() {
        let now = current_timestamp();
        
        // 30 seconds ago
        assert!(human_time_diff(now - 30).contains("seconds"));
        
        // 5 minutes ago
        assert!(human_time_diff(now - 300).contains("minutes"));
        
        // 2 hours ago
        assert!(human_time_diff(now - 7200).contains("hours"));
    }
}
