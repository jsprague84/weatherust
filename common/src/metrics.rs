/// Metrics infrastructure for tracking operations across all services
///
/// This module provides standardized metrics recording for:
/// - Notification delivery (success/failure rates)
/// - Operation execution times
/// - Service-specific counters

use metrics::{counter, histogram, gauge};

/// Record a notification send attempt
pub fn record_notification_sent(service: &str, backend: &str, success: bool) {
    let labels = [
        ("service", service.to_string()),
        ("backend", backend.to_string()),
        ("status", if success { "success" } else { "failure" }.to_string()),
    ];

    counter!("notifications_sent_total", &labels).increment(1);
}

/// Record operation execution time
pub fn record_operation_duration(service: &str, operation: &str, duration_secs: f64) {
    let labels = [
        ("service", service.to_string()),
        ("operation", operation.to_string()),
    ];

    histogram!("operation_duration_seconds", &labels).record(duration_secs);
}

/// Record a server operation result
pub fn record_server_operation(service: &str, server: &str, operation: &str, success: bool) {
    let labels = [
        ("service", service.to_string()),
        ("server", server.to_string()),
        ("operation", operation.to_string()),
        ("status", if success { "success" } else { "failure" }.to_string()),
    ];

    counter!("server_operations_total", &labels).increment(1);
}

/// Record Docker container health status
pub fn record_container_health(container: &str, health_status: &str, cpu_pct: Option<f64>, mem_pct: Option<f64>) {
    let labels = [
        ("container", container.to_string()),
        ("health", health_status.to_string()),
    ];

    counter!("container_health_checks_total", &labels).increment(1);

    if let Some(cpu) = cpu_pct {
        let cpu_labels = [("container", container.to_string())];
        gauge!("container_cpu_usage_percent", &cpu_labels).set(cpu);
    }

    if let Some(mem) = mem_pct {
        let mem_labels = [("container", container.to_string())];
        gauge!("container_memory_usage_percent", &mem_labels).set(mem);
    }
}

/// Record update check results
pub fn record_updates_available(server: &str, os_updates: usize, docker_updates: usize) {
    let os_labels = [
        ("server", server.to_string()),
        ("type", "os".to_string()),
    ];
    gauge!("updates_available", &os_labels).set(os_updates as f64);

    let docker_labels = [
        ("server", server.to_string()),
        ("type", "docker".to_string()),
    ];
    gauge!("updates_available", &docker_labels).set(docker_updates as f64);
}

/// Record speedtest results
pub fn record_speedtest_result(download_mbps: f64, upload_mbps: f64, ping_ms: f64, degraded: bool) {
    gauge!("speedtest_download_mbps").set(download_mbps);
    gauge!("speedtest_upload_mbps").set(upload_mbps);
    gauge!("speedtest_ping_ms").set(ping_ms);

    let labels = [("degraded", degraded.to_string())];
    counter!("speedtest_runs_total", &labels).increment(1);
}

/// Record weather API call
pub fn record_weather_fetch(success: bool, response_time_secs: f64) {
    let labels = [("status", if success { "success" } else { "failure" }.to_string())];
    counter!("weather_api_calls_total", &labels).increment(1);

    if success {
        histogram!("weather_api_response_time_seconds").record(response_time_secs);
    }
}

/// Record cleanup operation results
pub fn record_cleanup_operation(server: &str, cleanup_type: &str, items_removed: usize, bytes_reclaimed: Option<u64>) {
    let labels = [
        ("server", server.to_string()),
        ("type", cleanup_type.to_string()),
    ];

    counter!("cleanup_operations_total", &labels).increment(1);
    counter!("cleanup_items_removed_total", &labels).increment(items_removed as u64);

    if let Some(bytes) = bytes_reclaimed {
        histogram!("cleanup_bytes_reclaimed", &labels).record(bytes as f64);
    }
}

/// Record webhook request
pub fn record_webhook_request(endpoint: &str, status_code: u16, duration_secs: f64) {
    let labels = [
        ("endpoint", endpoint.to_string()),
        ("status", status_code.to_string()),
    ];

    counter!("webhook_requests_total", &labels).increment(1);
    histogram!("webhook_request_duration_seconds", &labels).record(duration_secs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_notification() {
        // Metrics recording should not panic
        record_notification_sent("test_service", "gotify", true);
        record_notification_sent("test_service", "ntfy", false);
    }

    #[test]
    fn test_record_operation_duration() {
        record_operation_duration("test_service", "fetch_weather", 1.5);
    }

    #[test]
    fn test_record_server_operation() {
        record_server_operation("updatectl", "server1", "os_update", true);
    }

    #[test]
    fn test_record_container_health() {
        record_container_health("nginx", "healthy", Some(25.5), Some(45.2));
    }

    #[test]
    fn test_record_updates_available() {
        record_updates_available("server1", 5, 3);
    }

    #[test]
    fn test_record_speedtest() {
        record_speedtest_result(100.0, 20.0, 15.5, false);
    }

    #[test]
    fn test_record_weather_fetch() {
        record_weather_fetch(true, 0.5);
    }

    #[test]
    fn test_record_cleanup() {
        record_cleanup_operation("server1", "docker_images", 10, Some(1024 * 1024 * 500));
    }

    #[test]
    fn test_record_webhook() {
        record_webhook_request("/health", 200, 0.01);
    }
}
