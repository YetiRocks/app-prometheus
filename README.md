<p align="center">
  <img src="https://cdn.prod.website-files.com/68e09cef90d613c94c3671c0/697e805a9246c7e090054706_logo_horizontal_grey.png" alt="Yeti" width="200" />
</p>

---

# app-prometheus

[![Yeti](https://img.shields.io/badge/Yeti-Application-blue)](https://yetirocks.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> **[Yeti](https://yetirocks.com)** - The Performance Platform for Agent-Driven Development.
> Schema-driven APIs, real-time streaming, and vector search. From prompt to production.

**Prometheus metrics exporter for yeti.** Industry-standard observability in one application.

app-prometheus exposes your entire yeti instance as a Prometheus scrape target -- health status, application inventory, telemetry record counts, and scrape timestamps in standard exposition format. Point Prometheus at one endpoint, build Grafana dashboards, set up alerting rules. No sidecars, no custom exporters, no glue code.

---

## Why app-prometheus

Yeti has built-in telemetry -- structured logs, spans, and metrics persisted in yeti-telemetry with real-time SSE streaming. That works for live debugging and ad-hoc inspection. But production monitoring needs Prometheus and Grafana: the industry standard for time-series alerting, dashboards, and long-term trend analysis.

Without app-prometheus, bridging the gap means writing a custom exporter, parsing internal APIs, formatting exposition text, and maintaining it as yeti evolves. That is plumbing work that every deployment repeats.

app-prometheus collapses all of that into a single yeti application:

- **Standard exposition format** -- `text/plain; version=0.0.4` compatible with Prometheus, Grafana Agent, Datadog Agent, Victoria Metrics, and any OpenMetrics-compatible scraper.
- **Zero configuration** -- install, restart yeti, point Prometheus at `/app-prometheus/api/metrics`. Done.
- **Fast mode** -- `?fast=true` skips expensive per-app and telemetry queries for high-frequency scraping (5s intervals).
- **Runtime configurable** -- toggle auth requirements, app metrics, telemetry collection, and custom labels via the ExporterSettings table. No restart required.
- **Native Rust plugin** -- compiles to a dylib, loads with yeti in seconds. No Node.js, no Python, no sidecar process.

---

## Quick Start

### 1. Install

```bash
cd ~/yeti/applications
git clone https://github.com/yetirocks/app-prometheus.git
```

Restart yeti. app-prometheus compiles automatically on first load (~2 minutes) and is cached for subsequent starts (~10 seconds).

### 2. Test the scrape endpoint

```bash
curl -k https://localhost:9996/app-prometheus/api/metrics \
  -H "Authorization: Bearer $TOKEN"
```

Example output:

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
yeti_application_info{app="app-cortex",version="0.1.0"} 1
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

### 3. Configure Prometheus

Add this to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'yeti'
    scrape_interval: 15s
    scheme: https
    tls_config:
      insecure_skip_verify: true    # for self-signed dev certs
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/api/metrics
    static_configs:
      - targets: ['localhost']
        labels:
          environment: 'production'
```

Restart Prometheus. Verify the target is UP in **Status > Targets**.

### 4. Add to Grafana

1. Open Grafana (default `http://localhost:3000`)
2. Go to **Connections > Data Sources > Add data source**
3. Select **Prometheus**, enter your Prometheus URL (e.g., `http://localhost:9090`)
4. Click **Save & Test**
5. Create a new dashboard and add panels using `yeti_*` metrics

---

## Architecture

```
Prometheus / Grafana Agent / Datadog Agent
    |
    |  GET /app-prometheus/api/metrics
    |  (scrape every 15s or 5s for fast mode)
    v
+-------------------------------------------------------+
|                  app-prometheus                        |
|                                                       |
|  metrics.rs                                           |
|  +--------------------------------------------------+ |
|  |                                                  | |
|  |  1. fetch("http://127.0.0.1/health")       | |
|  |     -> yeti_up, yeti_health_status,              | |
|  |        yeti_applications_total                   | |
|  |                                                  | |
|  |  2. fetch("http://127.0.0.1/admin/apps")   | |
|  |     -> yeti_application_info{app, version}       | |
|  |        (skipped in fast mode)                    | |
|  |                                                  | |
|  |  3. fetch(".../yeti-telemetry/Log?limit=0")      | |
|  |     -> yeti_telemetry_logs_total                 | |
|  |     fetch(".../yeti-telemetry/Span?limit=0")     | |
|  |     -> yeti_telemetry_spans_total                | |
|  |        (skipped in fast mode)                    | |
|  |                                                  | |
|  |  4. unix_timestamp()                             | |
|  |     -> yeti_scrape_timestamp_seconds             | |
|  +--------------------------------------------------+ |
|                                                       |
|  Format as Prometheus exposition text                  |
|  Content-Type: text/plain; version=0.0.4              |
+-------------------------------------------------------+
    |
    v
  Prometheus TSDB -> Grafana Dashboards -> Alertmanager
```

**Scrape path:** Prometheus sends `GET /app-prometheus/api/metrics` -> app-prometheus calls yeti internal APIs via `fetch()` (loopback HTTP) -> collects health, app inventory, and telemetry counts -> formats as Prometheus exposition text -> returns `text/plain; version=0.0.4`.

**Fast path:** `GET /app-prometheus/api/metrics?fast=true` -> skips steps 2 and 3 (per-app info and telemetry table scans) -> returns only core health metrics. Use this for high-frequency scraping without adding load.

---

## Features

### Metrics Endpoint (GET /metrics)

The primary endpoint. Returns all collected metrics in Prometheus exposition format:

```bash
curl -k https://localhost:9996/app-prometheus/api/metrics \
  -H "Authorization: Bearer $TOKEN"
```

Response content type: `text/plain; version=0.0.4; charset=utf-8`

Every metric includes `# HELP` and `# TYPE` comments per the Prometheus exposition format specification. The response ends with `# EOF`.

### Fast Mode (GET /metrics?fast=true)

Skips the two most expensive operations -- fetching the application list from `/admin/apps` and querying telemetry record counts from yeti-telemetry. Returns only:

- `yeti_up`
- `yeti_health_status`
- `yeti_applications_total` (from `/health` response, not per-app scan)
- `yeti_scrape_timestamp_seconds`

Use fast mode for high-frequency scraping (5-second intervals) to detect outages quickly without adding load to the telemetry subsystem.

```bash
curl -k "https://localhost:9996/app-prometheus/api/metrics?fast=true" \
  -H "Authorization: Bearer $TOKEN"
```

### Complete Metrics Reference

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `yeti_up` | gauge | -- | Always `1` when the instance is reachable. If the scrape fails, Prometheus marks it `0` automatically. |
| `yeti_health_status` | gauge | -- | `1` = healthy (health endpoint returned 200), `0` = degraded (non-200 response). |
| `yeti_applications_total` | gauge | -- | Number of loaded applications, extracted from the `/health` endpoint response. |
| `yeti_application_info` | gauge | `app`, `version` | Per-application metadata. One time series per loaded app. Value is always `1`; use labels for filtering and grouping. Skipped in fast mode. |
| `yeti_telemetry_logs_total` | gauge | -- | Total log records persisted in yeti-telemetry's Log table. Skipped in fast mode. |
| `yeti_telemetry_spans_total` | gauge | -- | Total span records persisted in yeti-telemetry's Span table. Skipped in fast mode. |
| `yeti_scrape_timestamp_seconds` | gauge | -- | Unix timestamp (seconds) when this scrape was generated. Useful for staleness detection and debugging scrape timing. |

### Full Example Scrape Output

Standard scrape with three applications loaded and telemetry active:

```
# HELP yeti_up Whether the yeti instance is up.
# TYPE yeti_up gauge
yeti_up 1
# HELP yeti_health_status Health check status (1=healthy).
# TYPE yeti_health_status gauge
yeti_health_status 1
# HELP yeti_applications_total Number of loaded applications.
# TYPE yeti_applications_total gauge
yeti_applications_total 8
# HELP yeti_application_info Application metadata.
# TYPE yeti_application_info gauge
yeti_application_info{app="app-cortex",version="0.1.0"} 1
yeti_application_info{app="app-prometheus",version="0.1.0"} 1
yeti_application_info{app="app-siem",version="0.1.0"} 1
yeti_application_info{app="demo-authentication",version="0.1.0"} 1
# HELP yeti_telemetry_logs_total Total log records.
# TYPE yeti_telemetry_logs_total gauge
yeti_telemetry_logs_total 152847
# HELP yeti_telemetry_spans_total Total span records.
# TYPE yeti_telemetry_spans_total gauge
yeti_telemetry_spans_total 38211
# HELP yeti_scrape_timestamp_seconds Unix timestamp of this scrape.
# TYPE yeti_scrape_timestamp_seconds gauge
yeti_scrape_timestamp_seconds 1743292800
# EOF
```

Fast mode scrape (same instance):

```
# HELP yeti_up Whether the yeti instance is up.
# TYPE yeti_up gauge
yeti_up 1
# HELP yeti_health_status Health check status (1=healthy).
# TYPE yeti_health_status gauge
yeti_health_status 1
# HELP yeti_applications_total Number of loaded applications.
# TYPE yeti_applications_total gauge
yeti_applications_total 8
# HELP yeti_scrape_timestamp_seconds Unix timestamp of this scrape.
# TYPE yeti_scrape_timestamp_seconds gauge
yeti_scrape_timestamp_seconds 1743292800
# EOF
```

---

## Data Model

### ExporterSettings Table

Runtime configuration for the metrics endpoint. Stored in the `app-prometheus` database.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | ID! (primary key) | `"default"` | Settings key. Use `"default"` for global settings. |
| `scrapeAuth` | String | `"true"` | Set to `"false"` to allow unauthenticated scrapes. Useful when Prometheus cannot send bearer tokens. |
| `includeAppMetrics` | String | `"true"` | Set to `"false"` to skip per-app info collection even in standard mode. |
| `includeTelemetry` | String | `"true"` | Set to `"false"` to skip telemetry log/span count collection even in standard mode. |
| `customLabels` | String (JSON) | -- | JSON object of key-value pairs to add as labels on all metrics. Example: `{"environment": "prod", "region": "us-east-1"}` |

Update settings at runtime via the auto-generated REST API:

```bash
curl -X POST https://localhost:9996/app-prometheus/api/ExporterSettings \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "default",
    "scrapeAuth": "true",
    "includeAppMetrics": "true",
    "includeTelemetry": "true",
    "customLabels": "{\"environment\": \"prod\", \"region\": \"us-east-1\"}"
  }'
```

---

## Configuration

### Cargo.toml ([package.metadata.app])

App configuration lives in `Cargo.toml` under `[package.metadata.app]`. There is no separate `config.yaml` or `services.yaml`:

```toml
[package]
name = "app-prometheus"
version = "0.1.0"
edition = "2024"
description = "Expose yeti metrics in Prometheus exposition format for Grafana and alerting"

[package.metadata.app]
schemas = "schemas/schema.graphql"
resources = "resources/*.rs"
```

To require authentication on the scrape endpoint, add a `[package.metadata.auth]` block:

```toml
[package.metadata.auth]
allow_signup = false
default_role = "scraper"
```

### Prometheus scrape_config -- Standard

Full metrics scrape every 15 seconds. Collects all metrics including per-app info and telemetry counts.

```yaml
scrape_configs:
  - job_name: 'yeti'
    scrape_interval: 15s
    scheme: https
    tls_config:
      insecure_skip_verify: true    # for self-signed dev certs
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/api/metrics
    static_configs:
      - targets: ['localhost']
        labels:
          environment: 'production'
```

### Prometheus scrape_config -- Fast

High-frequency health-only scrape. Skips expensive queries for rapid outage detection.

```yaml
scrape_configs:
  - job_name: 'yeti-fast'
    scrape_interval: 5s
    scheme: https
    tls_config:
      insecure_skip_verify: true
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/api/metrics
    params:
      fast: ['true']
    static_configs:
      - targets: ['localhost']
```

### Running Both Together

For production, run both scrape jobs simultaneously. Prometheus deduplicates overlapping metrics automatically:

```yaml
scrape_configs:
  # Full metrics every 15s
  - job_name: 'yeti'
    scrape_interval: 15s
    scheme: https
    tls_config:
      insecure_skip_verify: true
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/api/metrics
    static_configs:
      - targets: ['localhost']
        labels:
          environment: 'production'

  # Fast health check every 5s
  - job_name: 'yeti-fast'
    scrape_interval: 5s
    scheme: https
    tls_config:
      insecure_skip_verify: true
    bearer_token: '<your-jwt-token>'
    metrics_path: /app-prometheus/api/metrics
    params:
      fast: ['true']
    static_configs:
      - targets: ['localhost']
        labels:
          environment: 'production'
```

---

## Authentication

app-prometheus uses yeti's built-in auth system with JWT and Basic Auth methods configured in `Cargo.toml` under `[package.metadata.auth]`.

**Development mode:** All endpoints are accessible without authentication.

**Production mode:** The `/metrics` endpoint requires a valid JWT bearer token or Basic Auth credentials by default. Prometheus sends this via the `bearer_token` or `basic_auth` fields in `scrape_configs`.

**Disabling auth for scrapes:** If your Prometheus instance cannot send authentication headers (e.g., behind a reverse proxy that strips them), set `scrapeAuth` to `"false"` in ExporterSettings:

```bash
curl -X POST https://localhost:9996/app-prometheus/api/ExporterSettings \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"id": "default", "scrapeAuth": "false"}'
```

**Generating a JWT token for Prometheus:**

```bash
# Login to get a token
curl -X POST https://localhost:9996/yeti-auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-password"}'

# Use the access_token in prometheus.yml
# bearer_token: '<access_token from response>'
```

For long-lived scraping, use a dedicated service account with minimal permissions rather than an admin token.

---

## Grafana Integration

### Step-by-step Dashboard Setup

**1. Add Prometheus as a data source**

- Open Grafana at `http://localhost:3000`
- Navigate to **Connections > Data Sources > Add data source**
- Select **Prometheus**
- Set the URL to your Prometheus server (e.g., `http://localhost:9090`)
- Click **Save & Test** -- verify "Data source is working"

**2. Create a yeti dashboard**

- Click **Dashboards > New > New Dashboard**
- Click **Add visualization** and select your Prometheus data source

**3. Recommended panels**

| Panel Title | Query (PromQL) | Visualization | Description |
|-------------|----------------|---------------|-------------|
| Instance Status | `yeti_up` | Stat (green/red) | Shows whether yeti is reachable |
| Health Status | `yeti_health_status` | Stat (green/red) | 1 = healthy, 0 = degraded |
| Application Count | `yeti_applications_total` | Stat | Number of loaded applications |
| Applications | `yeti_application_info` | Table | List of all apps with version labels |
| Log Volume | `yeti_telemetry_logs_total` | Time series | Log record count over time |
| Span Volume | `yeti_telemetry_spans_total` | Time series | Span record count over time |
| Log Growth Rate | `rate(yeti_telemetry_logs_total[5m])` | Time series | Logs per second (5m average) |
| Scrape Freshness | `time() - yeti_scrape_timestamp_seconds` | Stat (with thresholds) | Seconds since last successful scrape |

**4. Set up alerts**

Example alert rules in Grafana:

| Alert | Condition | Severity |
|-------|-----------|----------|
| Yeti Down | `yeti_up == 0` for 1 minute | Critical |
| Health Degraded | `yeti_health_status == 0` for 2 minutes | Warning |
| No Apps Loaded | `yeti_applications_total == 0` for 5 minutes | Warning |
| Scrape Stale | `time() - yeti_scrape_timestamp_seconds > 60` | Warning |

**5. Import as JSON**

Export your dashboard as JSON via **Dashboard Settings > JSON Model** and commit it to version control for reproducible deployments.

---

## Project Structure

```
app-prometheus/
  Cargo.toml               # App configuration ([package.metadata.app] + optional [package.metadata.auth])
  schemas/
    schema.graphql         # ExporterSettings table definition
  resources/
    metrics.rs             # Prometheus exposition format endpoint
```

---

## Comparison

| | app-prometheus | Custom Exporter | Pushgateway | StatsD / Telegraf |
|---|---|---|---|---|
| **Deployment** | One yeti application, zero config | Custom code, separate process | Separate service + push logic | Agent + StatsD server |
| **Format** | Native Prometheus exposition | Must implement formatting | Native Prometheus | Requires translation layer |
| **Collection** | Pull-based (Prometheus scrapes) | Pull or push (your choice) | Push-based (app pushes metrics) | Push-based (UDP/TCP) |
| **Auth** | Built-in JWT/Basic via yeti-auth | Custom implementation | Separate auth layer | Network-level only |
| **Fast mode** | Built-in `?fast=true` | Custom implementation | N/A | N/A |
| **Runtime config** | ExporterSettings table, no restart | Code changes + redeploy | N/A | Config file + restart |
| **Maintenance** | Updates with yeti, schema-driven | Manual updates as APIs change | Separate version management | Separate version management |
| **Dependencies** | None (compiles to native Rust plugin) | Language runtime + libraries | Go binary | Go/C binary |

---

Built with [Yeti](https://yetirocks.com) | The Performance Platform for Agent-Driven Development
