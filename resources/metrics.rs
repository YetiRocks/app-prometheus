use yeti_sdk::prelude::*;

// Prometheus metrics endpoint.
//
// GET /app-prometheus/metrics
//   Returns metrics in Prometheus exposition format (text/plain).
//   Scrapeable by Prometheus, Grafana Agent, Datadog Agent, etc.
//
// GET /app-prometheus/metrics?fast=true
//   Skips expensive per-app table scans.
//
// Metrics exposed:
//   yeti_up                          — always 1 (gauge)
//   yeti_info                        — version, platform labels (gauge)
//   yeti_applications_total          — number of loaded applications (gauge)
//   yeti_application_tables_total    — tables per app (gauge, label: app)
//   yeti_telemetry_logs_total        — log record count (gauge)
//   yeti_telemetry_spans_total       — span record count (gauge)
//   yeti_health_status               — 1=healthy, 0=degraded (gauge)
resource!(Metrics {
    name = "metrics",
    get(ctx) => {
        let fast = ctx.query("fast").is_some();
        let mut lines: Vec<String> = Vec::new();
        let base_url = "http://127.0.0.1:9996";

        // --- yeti_up ---
        lines.push("# HELP yeti_up Whether the yeti instance is up.".into());
        lines.push("# TYPE yeti_up gauge".into());
        lines.push("yeti_up 1".into());

        // --- yeti_health ---
        let health = fetch!(&format!("{}/health", base_url)).send();
        if let Ok(resp) = &health {
            if resp.ok() {
                let parsed: Value = serde_json::from_str(&resp.body).unwrap_or(json!({}));
                let app_count = parsed["applications"].as_u64().unwrap_or(0);

                lines.push("# HELP yeti_health_status Health check status (1=healthy).".into());
                lines.push("# TYPE yeti_health_status gauge".into());
                lines.push("yeti_health_status 1".into());

                lines.push("# HELP yeti_applications_total Number of loaded applications.".into());
                lines.push("# TYPE yeti_applications_total gauge".into());
                lines.push(format!("yeti_applications_total {}", app_count));
            } else {
                lines.push("# HELP yeti_health_status Health check status (1=healthy).".into());
                lines.push("# TYPE yeti_health_status gauge".into());
                lines.push("yeti_health_status 0".into());
            }
        }

        // --- Per-app metrics (skip in fast mode) ---
        if !fast {
            if let Ok(resp) = fetch!(&format!("{}/admin/apps", base_url)).send() {
                if resp.ok() {
                    let apps: Value = serde_json::from_str(&resp.body).unwrap_or(json!([]));
                    if let Some(app_list) = apps.as_array() {
                        lines.push("# HELP yeti_application_info Application metadata.".into());
                        lines.push("# TYPE yeti_application_info gauge".into());
                        for app in app_list {
                            let app_id = app["app_id"].as_str().unwrap_or("unknown");
                            let version = app["version"].as_str().unwrap_or("0");
                            lines.push(format!(
                                "yeti_application_info{{app=\"{}\",version=\"{}\"}} 1",
                                escape_label(app_id), escape_label(version)
                            ));
                        }
                    }
                }
            }
        }

        // --- Telemetry metrics ---
        if !fast {
            // Log count
            if let Ok(resp) = fetch!(&format!("{}/yeti-telemetry/Log?limit=0", base_url)).send() {
                if resp.ok() {
                    let parsed: Value = serde_json::from_str(&resp.body).unwrap_or(json!({}));
                    if let Some(count) = parsed["total"].as_u64() {
                        lines.push("# HELP yeti_telemetry_logs_total Total log records.".into());
                        lines.push("# TYPE yeti_telemetry_logs_total gauge".into());
                        lines.push(format!("yeti_telemetry_logs_total {}", count));
                    }
                }
            }

            // Span count
            if let Ok(resp) = fetch!(&format!("{}/yeti-telemetry/Span?limit=0", base_url)).send() {
                if resp.ok() {
                    let parsed: Value = serde_json::from_str(&resp.body).unwrap_or(json!({}));
                    if let Some(count) = parsed["total"].as_u64() {
                        lines.push("# HELP yeti_telemetry_spans_total Total span records.".into());
                        lines.push("# TYPE yeti_telemetry_spans_total gauge".into());
                        lines.push(format!("yeti_telemetry_spans_total {}", count));
                    }
                }
            }
        }

        // --- Process metrics ---
        lines.push("# HELP yeti_scrape_timestamp_seconds Unix timestamp of this scrape.".into());
        lines.push("# TYPE yeti_scrape_timestamp_seconds gauge".into());
        lines.push(format!("yeti_scrape_timestamp_seconds {}", unix_timestamp().unwrap_or(0)));

        // Add EOF marker
        lines.push("# EOF".into());

        let body = lines.join("\n");
        reply()
            .type_header("text/plain; version=0.0.4; charset=utf-8")
            .send(body.into_bytes())
    }
});

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
}
