use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::state::AppState;

const CLASSES: [&str; 5] = ["1xx", "2xx", "3xx", "4xx", "5xx"];

#[derive(Debug, Default)]
pub struct Metrics {
    requests: [AtomicU64; 5],
}

impl Metrics {
    pub fn record(&self, status: u16) {
        let class = usize::from(status / 100)
            .saturating_sub(1)
            .min(CLASSES.len() - 1);
        self.requests[class].fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn render(&self, connections: u32, idle: usize) -> String {
        let open = usize::try_from(connections).unwrap_or(usize::MAX);
        let mut text = String::from(
            "# TYPE stashden_http_requests_total counter
",
        );
        for (class, count) in CLASSES.iter().zip(&self.requests) {
            let _ = writeln!(
                text,
                "stashden_http_requests_total{{class=\"{class}\"}} {}",
                count.load(Ordering::Relaxed)
            );
        }
        text.push_str(
            "# TYPE stashden_db_connections gauge
",
        );
        let _ = writeln!(
            text,
            "stashden_db_connections{{state=\"idle\"}} {idle}
stashden_db_connections{{state=\"busy\"}} {}",
            open.saturating_sub(idle)
        );
        text.push_str(
            "# TYPE stashden_build_info gauge
",
        );
        let _ = writeln!(
            text,
            "stashden_build_info{{version=\"{}\"}} 1",
            env!("CARGO_PKG_VERSION")
        );
        text
    }
}

pub async fn count(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    state.metrics.record(response.status().as_u16());
    response
}

#[must_use]
pub fn same_token(presented: &str, expected: &str) -> bool {
    presented.len() == expected.len()
        && presented
            .bytes()
            .zip(expected.bytes())
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_counted_by_status_class() {
        let metrics = Metrics::default();
        metrics.record(200);
        metrics.record(201);
        metrics.record(404);
        metrics.record(503);

        let text = metrics.render(5, 2);
        assert!(text.contains("stashden_http_requests_total{class=\"2xx\"} 2"));
        assert!(text.contains("stashden_http_requests_total{class=\"4xx\"} 1"));
        assert!(text.contains("stashden_http_requests_total{class=\"5xx\"} 1"));
        assert!(text.contains("stashden_http_requests_total{class=\"3xx\"} 0"));
        assert!(text.contains("stashden_db_connections{state=\"busy\"} 3"));
    }

    #[test]
    fn a_token_matches_only_itself() {
        assert!(same_token("s3cret-value", "s3cret-value"));
        assert!(!same_token("s3cret-valuf", "s3cret-value"));
        assert!(!same_token("s3cret", "s3cret-value"));
        assert!(!same_token("", "s3cret-value"));
    }
}
