/// Unit tests for speedynotify parsing functions

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::super::*;

    #[test]
    fn test_parse_ookla_json() {
        let sample_json = json!({
            "type": "result",
            "timestamp": "2024-01-15T10:30:00Z",
            "ping": {
                "jitter": 1.234,
                "latency": 15.5
            },
            "download": {
                "bandwidth": 125000000,
                "bytes": 50000000,
                "elapsed": 5000
            },
            "upload": {
                "bandwidth": 25000000,
                "bytes": 10000000,
                "elapsed": 4000
            },
            "isp": "Example ISP",
            "interface": {
                "name": "eth0"
            },
            "server": {
                "name": "TestServer",
                "location": "TestCity",
                "sponsor": "TestSponsor"
            }
        });

        let result: Result<OoklaResult, _> = serde_json::from_value(sample_json);
        assert!(result.is_ok());

        let ookla = result.unwrap();
        assert_eq!(ookla.ping.latency, 15.5);
        assert_eq!(ookla.download.bandwidth, 125000000.0);
        assert_eq!(ookla.upload.bandwidth, 25000000.0);
        assert_eq!(ookla.isp.unwrap(), "Example ISP");
        assert_eq!(ookla.interface.unwrap().name.unwrap(), "eth0");
        assert_eq!(ookla.server.as_ref().unwrap().name.as_ref().unwrap(), "TestServer");
        assert_eq!(ookla.server.unwrap().location.unwrap(), "TestCity");
    }

    #[test]
    fn test_parse_python_speedtest_json() {
        let sample_json = json!({
            "ping": 15.5,
            "download": 125000000.0,
            "upload": 25000000.0,
            "client": {
                "ip": "1.2.3.4",
                "isp": "Example ISP"
            },
            "server": {
                "name": "TestServer",
                "sponsor": "TestSponsor",
                "country": "US"
            }
        });

        let result: Result<PyResult, _> = serde_json::from_value(sample_json);
        assert!(result.is_ok());

        let python = result.unwrap();
        assert_eq!(python.ping, 15.5);
        assert_eq!(python.download, 125000000.0);
        assert_eq!(python.upload, 25000000.0);
        assert_eq!(python.client.unwrap().isp.unwrap(), "Example ISP");
        assert_eq!(python.server.unwrap().name.unwrap(), "TestServer");
    }

    #[test]
    fn test_bandwidth_to_mbps() {
        // Ookla returns bandwidth in bytes per second
        // 125000000 bytes/sec = 1000 Mbps
        let bandwidth = 125000000u64;
        let mbps = (bandwidth as f64) * 8.0 / 1_000_000.0;
        assert_eq!(mbps, 1000.0);
    }

    #[test]
    fn test_python_download_to_mbps() {
        // Python speedtest-cli returns bits per second
        // 125000000 bps = 125 Mbps
        let download = 125000000.0f64;
        let mbps = download / 1_000_000.0;
        assert_eq!(mbps, 125.0);
    }

    #[test]
    fn test_degraded_detection() {
        // Test logic for degraded speed detection
        let down_mbps = 50.0;
        let up_mbps = 10.0;
        let min_down = Some(100.0);
        let min_up = Some(20.0);

        let degraded = min_down.map(|min| down_mbps < min).unwrap_or(false)
            || min_up.map(|min| up_mbps < min).unwrap_or(false);

        assert!(degraded, "Should detect degraded speeds when below thresholds");
    }

    #[test]
    fn test_no_degradation_when_above_thresholds() {
        let down_mbps = 150.0;
        let up_mbps = 25.0;
        let min_down = Some(100.0);
        let min_up = Some(20.0);

        let degraded = min_down.map(|min| down_mbps < min).unwrap_or(false)
            || min_up.map(|min| up_mbps < min).unwrap_or(false);

        assert!(!degraded, "Should not detect degradation when above thresholds");
    }

    #[test]
    fn test_no_degradation_when_thresholds_not_set() {
        let down_mbps = 10.0;
        let up_mbps = 5.0;
        let min_down: Option<f64> = None;
        let min_up: Option<f64> = None;

        let degraded = min_down.map(|min| down_mbps < min).unwrap_or(false)
            || min_up.map(|min| up_mbps < min).unwrap_or(false);

        assert!(!degraded, "Should not detect degradation when thresholds not set");
    }
}
