// Metrics module for MyApp Controller
// Provides Prometheus metrics for monitoring controller performance

use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, Encoder,
    GaugeVec, HistogramVec, TextEncoder,
};
use std::time::Instant;
use warp::{Filter, Reply};

// Metric definitions
lazy_static::lazy_static! {
    // Reconciliation metrics
    static ref RECONCILE_COUNTER: CounterVec = register_counter_vec!(
        "myapp_reconcile_total",
        "Total number of reconciliation attempts",
        &["namespace", "name", "result"]
    ).unwrap();

    static ref RECONCILE_DURATION: HistogramVec = register_histogram_vec!(
        "myapp_reconcile_duration_seconds",
        "Time spent in reconciliation",
        &["namespace", "name"],
        vec![0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0]
    ).unwrap();

    // Resource metrics
    static ref MANAGED_RESOURCES: GaugeVec = register_gauge_vec!(
        "myapp_managed_resources_total",
        "Number of resources managed by controller",
        &["resource_type", "namespace"]
    ).unwrap();

    // Error metrics
    static ref ERROR_COUNTER: CounterVec = register_counter_vec!(
        "myapp_errors_total",
        "Total number of errors by type",
        &["error_type", "namespace"]
    ).unwrap();

    // Webhook metrics
    static ref WEBHOOK_COUNTER: CounterVec = register_counter_vec!(
        "myapp_webhook_requests_total",
        "Total webhook requests",
        &["webhook_type", "result"]
    ).unwrap();

    static ref WEBHOOK_DURATION: HistogramVec = register_histogram_vec!(
        "myapp_webhook_duration_seconds",
        "Webhook request duration",
        &["webhook_type"],
        vec![0.001, 0.01, 0.1, 0.5, 1.0]
    ).unwrap();

    // Controller health metrics
    static ref CONTROLLER_INFO: GaugeVec = register_gauge_vec!(
        "myapp_controller_info",
        "Controller version and build info",
        &["version", "build_date", "git_commit"]
    ).unwrap();

    static ref ACTIVE_RECONCILES: GaugeVec = register_gauge_vec!(
        "myapp_active_reconciles",
        "Number of active reconciliation loops",
        &["namespace"]
    ).unwrap();
}

/// Metrics collector for tracking controller performance
pub struct MetricsCollector {
    start_time: Instant,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        // Initialize controller info metric
        CONTROLLER_INFO
            .with_label_values(&[
                env!("CARGO_PKG_VERSION"),
                &std::env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string()),
                &std::env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
            ])
            .set(1.0);

        Self {
            start_time: Instant::now(),
        }
    }

    /// Start timing a reconciliation
    pub fn start_reconcile(&self, namespace: &str, name: &str) -> ReconcileTimer {
        ACTIVE_RECONCILES.with_label_values(&[namespace]).inc();
        ReconcileTimer {
            namespace: namespace.to_string(),
            name: name.to_string(),
            start: Instant::now(),
        }
    }

    /// Record error occurrence
    pub fn record_error(&self, error_type: &str, namespace: &str) {
        ERROR_COUNTER
            .with_label_values(&[error_type, namespace])
            .inc();
    }

    /// Update managed resource count
    pub fn set_managed_resources(&self, resource_type: &str, namespace: &str, count: i64) {
        MANAGED_RESOURCES
            .with_label_values(&[resource_type, namespace])
            .set(count as f64);
    }

    /// Start timing a webhook request
    pub fn start_webhook(&self, webhook_type: &str) -> WebhookTimer {
        WebhookTimer {
            webhook_type: webhook_type.to_string(),
            start: Instant::now(),
        }
    }

    /// Get controller uptime in seconds
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

/// Timer for tracking reconciliation duration
pub struct ReconcileTimer {
    namespace: String,
    name: String,
    start: Instant,
}

impl ReconcileTimer {
    /// Complete the reconciliation with success
    pub fn success(self) {
        let duration = self.start.elapsed().as_secs_f64();

        RECONCILE_COUNTER
            .with_label_values(&[&self.namespace, &self.name, "success"])
            .inc();

        RECONCILE_DURATION
            .with_label_values(&[&self.namespace, &self.name])
            .observe(duration);

        ACTIVE_RECONCILES
            .with_label_values(&[&self.namespace])
            .dec();
    }

    /// Complete the reconciliation with error
    pub fn error(self, error_type: &str) {
        let duration = self.start.elapsed().as_secs_f64();

        RECONCILE_COUNTER
            .with_label_values(&[&self.namespace, &self.name, "error"])
            .inc();

        RECONCILE_DURATION
            .with_label_values(&[&self.namespace, &self.name])
            .observe(duration);

        ERROR_COUNTER
            .with_label_values(&[error_type, &self.namespace])
            .inc();

        ACTIVE_RECONCILES
            .with_label_values(&[&self.namespace])
            .dec();
    }
}

/// Timer for tracking webhook duration
pub struct WebhookTimer {
    webhook_type: String,
    start: Instant,
}

impl WebhookTimer {
    /// Complete the webhook request with success
    pub fn success(self) {
        let duration = self.start.elapsed().as_secs_f64();

        WEBHOOK_COUNTER
            .with_label_values(&[&self.webhook_type, "success"])
            .inc();

        WEBHOOK_DURATION
            .with_label_values(&[&self.webhook_type])
            .observe(duration);
    }

    /// Complete the webhook request with error
    pub fn error(self) {
        let duration = self.start.elapsed().as_secs_f64();

        WEBHOOK_COUNTER
            .with_label_values(&[&self.webhook_type, "error"])
            .inc();

        WEBHOOK_DURATION
            .with_label_values(&[&self.webhook_type])
            .observe(duration);
    }
}

/// Create metrics endpoint for Prometheus scraping
pub fn metrics_handler() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path("metrics")
        .and(warp::get())
        .map(|| {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        })
        .map(|metrics: String| {
            warp::reply::with_header(
                metrics,
                "content-type",
                "text/plain; version=0.0.4; charset=utf-8",
            )
        })
}

/// Health check endpoint
pub fn health_handler() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION")
        }))
    })
}

/// Readiness check endpoint
pub fn ready_handler() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path("ready").and(warp::get()).map(|| {
        // Add readiness checks here (e.g., Kubernetes API connectivity)
        warp::reply::json(&serde_json::json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // Test reconcile timing
        let timer = collector.start_reconcile("default", "test-app");
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.success();

        // Test error recording
        collector.record_error("validation_error", "default");

        // Test resource counting
        collector.set_managed_resources("deployment", "default", 5);

        // Verify metrics exist (basic smoke test)
        let metrics = prometheus::gather();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_webhook_timing() {
        let collector = MetricsCollector::new();

        let timer = collector.start_webhook("validate");
        std::thread::sleep(std::time::Duration::from_millis(5));
        timer.success();

        let timer = collector.start_webhook("mutate");
        timer.error();
    }
}
