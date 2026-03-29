<p align="center">
  <img src="https://cdn.prod.website-files.com/68e09cef90d613c94c3671c0/697e805a9246c7e090054706_logo_horizontal_grey.png" alt="Yeti" width="200" />
</p>

---

# app-prometheus

[![Yeti](https://img.shields.io/badge/Yeti-Application-blue)](https://yetirocks.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> **[Yeti](https://yetirocks.com)** - The Performance Platform for Agent-Driven Development.
> Schema-driven APIs, real-time streaming, and vector search. From prompt to production.

Prometheus metrics exporter for yeti -- exposes health, application, and telemetry metrics in Prometheus exposition format.

## Features

- **/metrics endpoint** in standard Prometheus exposition format (`text/plain; version=0.0.4`)
- **Fast mode** (`?fast=true`) skips expensive per-app and telemetry table scans
- **Health metrics** -- instance up status and health check result
- **Application metrics** -- total count, per-app info with version labels
- **Telemetry metrics** -- log and span record counts from yeti-telemetry
- **Scrape timestamp** for staleness detection
- **Configurable** via ExporterSettings table (auth, app metrics, telemetry, custom labels)

## Installation

```bash
git clone https://github.com/yetirocks/app-prometheus.git
cp -r app-prometheus ~/yeti/applications/
```

## Project Structure

```
app-prometheus/
  config.yaml
  schemas/
    schema.graphql
  resources/
    metrics.rs      # Prometheus exposition format endpoint
```

## Configuration

```yaml
name: "Prometheus Exporter"
app_id: "app-prometheus"
version: "0.1.0"
description: "Expose yeti metrics in Prometheus exposition format for Grafana and alerting"

schemas:
  - schemas/schema.graphql

resources:
  - resources/*.rs

auth:
  methods: [jwt, basic]
```

## Schema

**ExporterSettings** -- Runtime configuration for the metrics endpoint.

```graphql
type ExporterSettings @table(database: "app-prometheus") @export {
    id: ID! @primaryKey              # "default"
    scrapeAuth: String               # "true" to require auth on /metrics (default "true")
    includeAppMetrics: String        # "true" to include per-app table counts (default "true")
    includeTelemetry: String         # "true" to include telemetry log/span counts (default "true")
    customLabels: String             # JSON: {"environment": "prod", "region": "us-east-1"}
}
```

## API Reference

### GET /app-prometheus/metrics

Returns all metrics in Prometheus exposition format.

```bash
curl https://localhost:9996/app-prometheus/metrics \
  -H "Authorization: Bearer $TOKEN"
```

**Response** (`text/plain; version=0.0.4`):

```
# HELP yeti_up Whether the yeti instance is up.
# TYPE yeti_up gauge
yeti_up 1
# HELP yeti_health_status Health check status (1=healthy).
# TYPE yeti_health_status gauge
yeti_health_status 1
# HELP yeti_applications_total Number of loaded applications.
# TYPE yeti_applications_total gauge
yeti_applications_total 12
# HELP yeti_application_info Application metadata.
# TYPE yeti_application_info gauge
yeti_application_info{app="app-siem",version="0.1.0"} 1
yeti_application_info{app="app-prometheus",version="0.1.0"} 1
# HELP yeti_telemetry_logs_total Total log records.
# TYPE yeti_telemetry_logs_total gauge
yeti_telemetry_logs_total 48231
# HELP yeti_telemetry_spans_total Total span records.
# TYPE yeti_telemetry_spans_total gauge
yeti_telemetry_spans_total 12044
# HELP yeti_scrape_timestamp_seconds Unix timestamp of this scrape.
# TYPE yeti_scrape_timestamp_seconds gauge
yeti_scrape_timestamp_seconds 1711700000
# EOF
```

### GET /app-prometheus/metrics?fast=true

Skip per-app info and telemetry table counts. Returns only `yeti_up`, `yeti_health_status`, `yeti_applications_total`, and `yeti_scrape_timestamp_seconds`.

```bash
curl "https://localhost:9996/app-prometheus/metrics?fast=true" \
  -H "Authorization: Bearer $TOKEN"
```

### Metrics Reference

| Metric | Type | Description |
|--------|------|-------------|
| `yeti_up` | gauge | Always 1 when the instance is reachable |
| `yeti_health_status` | gauge | 1 = healthy, 0 = degraded |
| `yeti_applications_total` | gauge | Number of loaded applications |
| `yeti_application_info` | gauge | Per-app metadata (labels: `app`, `version`) |
| `yeti_telemetry_logs_total` | gauge | Total log records in yeti-telemetry |
| `yeti_telemetry_spans_total` | gauge | Total span records in yeti-telemetry |
| `yeti_scrape_timestamp_seconds` | gauge | Unix timestamp of this scrape |

## Prometheus Scrape Configuration

Add this to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'yeti'
    scrape_interval: 15s
    scheme: https
    tls_config:
      insecure_skip_verify: true    # for self-signed dev certs
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/metrics
    static_configs:
      - targets: ['localhost:9996']
        labels:
          environment: 'production'

  # Fast scrape for high-frequency monitoring (skips expensive queries)
  - job_name: 'yeti-fast'
    scrape_interval: 5s
    scheme: https
    tls_config:
      insecure_skip_verify: true
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/metrics
    params:
      fast: ['true']
    static_configs:
      - targets: ['localhost:9996']
```

---

Built with [Yeti](https://yetirocks.com) | The Performance Platform for Agent-Driven Development
