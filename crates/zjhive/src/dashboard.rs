//! Host-independent state transitions keep read-only and freshness guarantees testable.

use crate::model::{self, Catalog, DashboardError, Session, Status};
use std::collections::BTreeMap;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Catalog,
    Status,
}

impl Endpoint {
    pub const ALL: [Self; 2] = [Self::Catalog, Self::Status];

    fn index(self) -> usize {
        match self {
            Self::Catalog => 0,
            Self::Status => 1,
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Catalog => "v1/sessions",
            Self::Status => "v1/status",
        }
    }

    fn interval(self) -> u64 {
        match self {
            Self::Catalog => 4,
            Self::Status => 20,
        }
    }
}

#[derive(Default)]
struct Fetch {
    latest: u64,
    active: Option<u64>,
    success: Option<u64>,
    error: Option<String>,
    failures: u32,
    next: u64,
}

struct Request {
    endpoint: Endpoint,
    started: u64,
}

pub struct Dashboard {
    base: Result<url::Url, DashboardError>,
    pub permission: String,
    migration: bool,
    catalog: Option<Catalog>,
    status: Option<Status>,
    selected: usize,
    details: bool,
    scroll: usize,
    notice: Option<String>,
    fetch: [Fetch; 2],
    serial: u64,
    requests: BTreeMap<u64, Request>,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new("http://127.0.0.1:8765", false)
    }
}

impl Dashboard {
    pub fn new(url: &str, migration: bool) -> Self {
        Self {
            base: model::base_url(url),
            permission: "Waiting for WebAccess permission".into(),
            migration,
            catalog: None,
            status: None,
            selected: 0,
            details: false,
            scroll: 0,
            notice: None,
            fetch: Default::default(),
            serial: 0,
            requests: BTreeMap::new(),
        }
    }

    pub fn request(&mut self, endpoint: Endpoint, now: u64, manual: bool) -> Option<(u64, String)> {
        let url = self.base.as_ref().ok()?.join(endpoint.path()).ok()?;
        let count = self
            .requests
            .values()
            .filter(|r| r.endpoint == endpoint)
            .count();
        let fetch = &mut self.fetch[endpoint.index()];
        // Expiry cannot cancel host I/O: permit one replacement, then wait for a real callback.
        if count >= 2 || fetch.active.is_some() || (!manual && now < fetch.next) {
            return None;
        }
        self.serial += 1;
        fetch.latest = self.serial;
        fetch.active = Some(self.serial);
        self.requests.insert(
            self.serial,
            Request {
                endpoint,
                started: now,
            },
        );
        Some((self.serial, url.into()))
    }

    fn failure(&mut self, endpoint: Endpoint, message: String, now: u64) {
        let fetch = &mut self.fetch[endpoint.index()];
        fetch.active = None;
        fetch.error = Some(message);
        fetch.failures = (fetch.failures + 1).min(6);
        fetch.next = now + (1_u64 << fetch.failures).min(60);
    }

    pub fn expire(&mut self, now: u64) {
        for endpoint in Endpoint::ALL {
            if self.fetch[endpoint.index()].active.is_some_and(|serial| {
                self.requests
                    .get(&serial)
                    .is_some_and(|r| now.saturating_sub(r.started) >= 15)
            }) {
                self.failure(endpoint, DashboardError::Overdue.to_string(), now);
            }
        }
    }

    pub fn response(&mut self, serial: u64, status: u16, body: &[u8], now: u64) {
        let Some(request) = self.requests.remove(&serial) else {
            return;
        };
        let endpoint = request.endpoint;
        let fetch = &self.fetch[endpoint.index()];
        // A retired request may release capacity, but cannot overwrite a newer snapshot.
        if fetch.latest != serial || fetch.active != Some(serial) {
            return;
        }
        let result = if status != 200 {
            Err(DashboardError::Http(status))
        } else {
            match endpoint {
                Endpoint::Catalog => {
                    model::catalog(body).map(|catalog| self.accept_catalog(catalog))
                }
                Endpoint::Status => serde_json::from_slice::<Status>(body)
                    .map(|status| self.status = Some(status))
                    .map_err(|e| DashboardError::Response(e.to_string())),
            }
        };
        match result {
            Ok(()) => {
                self.fetch[endpoint.index()] = Fetch {
                    latest: serial,
                    success: Some(now),
                    next: now + endpoint.interval(),
                    ..Default::default()
                };
            }
            Err(error) => self.failure(endpoint, error.to_string(), now),
        }
    }

    fn accept_catalog(&mut self, catalog: Catalog) {
        let previous = self.session().map(|s| s.session_id.clone());
        if let Some(id) = previous {
            if let Some(index) = catalog.sessions.iter().position(|s| s.session_id == id) {
                self.selected = index;
            } else {
                self.details = false;
                self.scroll = 0;
                self.notice = Some("Selected session is no longer listed".into());
            }
        }
        self.selected = self.selected.min(catalog.sessions.len().saturating_sub(1));
        self.catalog = Some(catalog);
    }

    fn session(&self) -> Option<&Session> {
        self.catalog.as_ref()?.sessions.get(self.selected)
    }

    pub fn navigate(&mut self, delta: isize) {
        if self.details {
            self.scroll = self.scroll.saturating_add_signed(delta);
        } else if let Some(catalog) = &self.catalog {
            self.selected = self
                .selected
                .saturating_add_signed(delta)
                .min(catalog.sessions.len().saturating_sub(1));
            self.notice = None;
        }
    }

    pub fn home(&mut self) {
        if self.details {
            self.scroll = 0;
        } else {
            self.selected = 0;
        }
    }

    pub fn end(&mut self) {
        if self.details {
            self.scroll = usize::MAX;
        } else {
            self.selected = self
                .catalog
                .as_ref()
                .map_or(0, |c| c.sessions.len().saturating_sub(1));
        }
    }

    pub fn open_details(&mut self) {
        self.details = self.session().is_some();
        self.scroll = 0;
    }

    pub fn close_details(&mut self) {
        self.details = false;
        self.scroll = 0;
    }

    fn freshness(&self, endpoint: Endpoint, now: u64) -> String {
        let fetch = &self.fetch[endpoint.index()];
        match (&fetch.error, fetch.success) {
            (Some(error), Some(tick)) => {
                format!("STALE ({}s old): {error}", now.saturating_sub(tick))
            }
            (Some(error), None) => format!("Unavailable: {error}"),
            (None, Some(tick)) => format!("Fresh ({}s old)", now.saturating_sub(tick)),
            (None, None) => "Loading".into(),
        }
    }

    fn detail_lines(&self, s: &Session) -> Vec<String> {
        let mut lines = vec![
            format!("Session: {}", s.session_id),
            format!("Title: {}", text(&s.title)),
            format!("Agent: {} | cwd: {}", s.agent, s.cwd),
            format!(
                "Lifecycle: {} | Recovery: {}",
                text(&s.lifecycle),
                text(&s.recovery)
            ),
            format!("Recovery reason: {}", text(&s.recovery_reason)),
            format!(
                "Activity: {} | Last activity (Unix ms): {}",
                text(&s.activity),
                number(s.last_activity_at)
            ),
            format!(
                "Control owner: {}",
                s.control_owner.as_deref().unwrap_or("free / not reported")
            ),
            format!(
                "Generation: {} | Observers: {}",
                number(s.generation),
                number(s.observers)
            ),
            format!(
                "Origin: {} | Native ID: {}",
                text(&s.origin),
                text(&s.native_id)
            ),
            format!(
                "External activity: {}",
                s.external_activity
                    .map_or("unknown", |v| if v { "yes" } else { "no" })
            ),
        ];
        if let Some(usage) = &s.usage {
            lines.push(format!(
                "Tokens: {} | Input: {} | Output: {}",
                number(usage.tokens.total_tokens),
                number(usage.tokens.input_tokens),
                number(usage.tokens.output_tokens)
            ));
            lines.push(format!(
                "Context: {} / {}",
                number(usage.context_used),
                number(usage.context_size)
            ));
            lines.push(format!(
                "Estimated cost: {} | Actual session cost: {}",
                cost(&usage.estimated_cost),
                cost(&usage.actual_session_cost)
            ));
        } else {
            lines.push("Usage: unknown".into());
        }
        if let Ok(base) = &self.base
            && let Ok(mut url) = base.join("ui")
        {
            url.query_pairs_mut().append_pair("session", &s.session_id);
            lines.push(format!("WebUI: {url}"));
        }
        lines.push("Metadata only; use Hive TUI/WebUI for messages and control".into());
        lines
    }

    pub fn render(&mut self, rows: usize, cols: usize, now: u64) -> Vec<String> {
        if rows == 0 || cols == 0 {
            return vec![];
        }
        let mut header = vec!["ZJHive · read-only dashboard".into()];
        match &self.base {
            Ok(url) => header.push(format!("Hive: {url} | {}", self.permission)),
            Err(error) => header.push(error.to_string()),
        }
        header.push(format!(
            "Catalogue: {}",
            self.freshness(Endpoint::Catalog, now)
        ));
        header.push(format!("Status: {}", self.freshness(Endpoint::Status, now)));
        if let Some(status) = &self.status {
            let profiles = status
                .active_by_agent
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(", ");
            header.push(format!(
                "{} {} | Sessions: {} | Default: {} | Live sessions by agent: {}",
                status.name,
                status.version,
                status.sessions,
                text(&status.default_agent),
                profiles
            ));
        }
        if self.migration {
            header.push("Migration: bridge setting ignored; no companion is needed".into());
        }
        if let Some(notice) = &self.notice {
            header.push(notice.clone());
        }
        let footer = if self.details {
            "↑/↓ scroll · Home/End · Esc catalogue · r refresh"
        } else {
            "↑/↓ select · PgUp/PgDn · Home/End · Enter details · r refresh"
        };
        // Reserve the footer even in short panes; long lists never hide navigation.
        header.truncate(rows.saturating_sub(1));
        let capacity = rows.saturating_sub(header.len() + 1);
        let body = if self.details {
            self.session()
                .map(|s| self.detail_lines(s))
                .unwrap_or_default()
                .into_iter()
                // Metadata paths and IDs must remain inspectable even in narrow panes.
                .flat_map(|line| wrap_detail(&line, cols))
                .collect()
        } else if let Some(catalog) = &self.catalog {
            if catalog.sessions.is_empty() {
                vec!["No sessions listed; create/manage sessions in Hive TUI/WebUI".into()]
            } else {
                catalog
                    .sessions
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        format!(
                            "{} {} | {} | {} / {} | {} | control: {}",
                            if i == self.selected { ">" } else { " " },
                            s.title.as_deref().unwrap_or(&s.session_id),
                            s.agent,
                            text(&s.lifecycle),
                            text(&s.recovery),
                            text(&s.activity),
                            if s.control_owner.is_some() {
                                "occupied"
                            } else {
                                "free / not reported"
                            }
                        )
                    })
                    .collect()
            }
        } else {
            vec!["Catalogue not yet available".into()]
        };
        let start = if self.details {
            self.scroll = self.scroll.min(body.len().saturating_sub(capacity.max(1)));
            self.scroll
        } else {
            self.selected.saturating_sub(capacity.saturating_sub(1))
        };
        header.extend(body.into_iter().skip(start).take(capacity));
        if let Some(catalog) = &self.catalog
            && header.len() < rows.saturating_sub(1)
        {
            header.push(format!("Catalogue revision: {}", catalog.revision));
        }
        header.push(footer.into());
        header
            .into_iter()
            .map(|line| safe_line(&line, cols))
            .collect()
    }
}

fn text(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("unknown")
}
fn number(value: Option<u64>) -> String {
    value.map_or_else(|| "unknown".into(), |v| v.to_string())
}
fn cost(value: &Option<model::Cost>) -> String {
    value.as_ref().map_or_else(
        || "unknown".into(),
        |c| format!("{} {}", c.amount, c.currency),
    )
}

fn wrap_detail(line: &str, cols: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut current = String::new();
    let mut width = 0;
    for c in line.chars().filter(|c| !c.is_control()) {
        // A double-width glyph cannot fit a one-column pane; preserve progress visibly.
        let c = if c.width().unwrap_or(0) > cols {
            '?'
        } else {
            c
        };
        let size = c.width().unwrap_or(0);
        if width + size > cols && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            width = 0;
        }
        current.push(c);
        width += size;
    }
    lines.push(current);
    lines
}

fn safe_line(line: &str, cols: usize) -> String {
    let mut width = 0;
    line.chars()
        .filter(|c| !c.is_control())
        .take_while(|c| {
            width += c.width().unwrap_or(0);
            width <= cols
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(ids: &[&str], revision: u64) -> Vec<u8> {
        serde_json::to_vec(
            &json!({"revision": revision, "sessions": ids.iter().map(|id|
            json!({"sessionId": id, "agent": "test", "cwd": "/tmp", "activity": "future-state"})
        ).collect::<Vec<_>>(), "newField": true}),
        )
        .unwrap()
    }

    fn receive(d: &mut Dashboard, ids: &[&str], revision: u64, now: u64) {
        let (serial, _) = d.request(Endpoint::Catalog, now, true).unwrap();
        d.response(serial, 200, &snapshot(ids, revision), now);
    }

    #[test]
    fn selection_survives_reorder_and_a_lower_revision_after_restart() {
        let mut d = Dashboard::default();
        receive(&mut d, &["a", "b"], 10, 0);
        d.navigate(1);
        d.open_details();
        receive(&mut d, &["b", "a"], 1, 4);
        assert_eq!(d.session().unwrap().session_id, "b");
        assert!(d.details);
        receive(&mut d, &["a"], 2, 8);
        assert!(!d.details);
        assert!(d.notice.unwrap().contains("no longer listed"));
    }

    #[test]
    fn invalid_catalogues_preserve_cached_data_and_status_cannot_clear_staleness() {
        let mut d = Dashboard::default();
        receive(&mut d, &["a"], 1, 0);
        let (serial, _) = d.request(Endpoint::Catalog, 4, true).unwrap();
        d.response(serial, 200, br#"{"sessions":[]}"#, 4);
        let (serial, _) = d.request(Endpoint::Status, 4, true).unwrap();
        d.response(
            serial,
            200,
            br#"{"name":"hive","version":"1","sessions":1}"#,
            4,
        );
        assert!(d.freshness(Endpoint::Catalog, 4).contains("STALE"));
        assert!(d.freshness(Endpoint::Status, 4).contains("Fresh"));
        assert_eq!(d.session().unwrap().session_id, "a");
        receive(&mut d, &[], 2, 8);
        assert!(d.freshness(Endpoint::Catalog, 8).contains("Fresh"));
        assert!(
            d.render(20, 120, 8)
                .join("\n")
                .contains("No sessions listed")
        );
    }

    #[test]
    fn overdue_requests_allow_one_replacement_and_late_results_cannot_overwrite_it() {
        let mut d = Dashboard::default();
        let (old, _) = d.request(Endpoint::Catalog, 0, false).unwrap();
        d.expire(15);
        assert!(d.request(Endpoint::Catalog, 16, false).is_none());
        let (new, _) = d.request(Endpoint::Catalog, 17, false).unwrap();
        d.expire(32);
        assert!(d.request(Endpoint::Catalog, 100, true).is_none());
        d.response(old, 200, &snapshot(&["old"], 1), 100);
        assert!(d.catalog.is_none());
        // The second expired response also releases capacity without accepting stale data.
        d.response(new, 200, &snapshot(&["expired"], 2), 100);
        assert!(d.catalog.is_none());
        receive(&mut d, &["fresh"], 3, 101);
        assert_eq!(d.session().unwrap().session_id, "fresh");
        assert!(d.fetch[0].failures == 0);
    }

    #[test]
    fn details_are_local_and_unknown_usage_is_never_rendered_as_zero() {
        let mut d = Dashboard::default();
        receive(&mut d, &["a"], 1, 0);
        d.open_details();
        assert!(d.requests.is_empty());
        let screen = d.render(30, 160, 0).join("\n");
        assert!(screen.contains("Usage: unknown"));
        assert!(screen.contains("future-state"));
        assert!(screen.contains("Metadata only"));
        assert!(screen.contains("/ui?session=a"));
    }

    #[test]
    fn long_lists_keep_selection_and_footer_visible_and_tiny_panes_are_safe() {
        let mut d = Dashboard::default();
        let ids: Vec<_> = (0..50).map(|n| format!("session-{n}")).collect();
        let refs: Vec<_> = ids.iter().map(String::as_str).collect();
        receive(&mut d, &refs, 1, 0);
        d.end();
        let screen = d.render(12, 100, 0);
        assert!(screen.iter().any(|l| l.starts_with("> session-49")));
        assert!(screen.last().unwrap().contains("r refresh"));
        for rows in 0..8 {
            assert!(d.render(rows, 1, 0).len() <= rows);
        }
        assert_eq!(safe_line("a\x1b\n界b", 3), "a界");
    }

    #[test]
    fn only_operational_endpoints_are_generated_and_unsafe_urls_are_rejected() {
        let mut d = Dashboard::new("https://example.org/hive", false);
        for (endpoint, expected) in [
            (Endpoint::Catalog, "sessions"),
            (Endpoint::Status, "status"),
        ] {
            let (_, url) = d.request(endpoint, 0, true).unwrap();
            assert_eq!(url, format!("https://example.org/hive/v1/{expected}"));
        }
        for url in [
            "file:///tmp",
            "http://user:pass@localhost",
            "http://localhost/?x=1",
            "http://localhost/#x",
        ] {
            assert!(
                Dashboard::new(url, false)
                    .request(Endpoint::Catalog, 0, true)
                    .is_none()
            );
        }
    }

    #[test]
    fn malformed_and_duplicate_identities_do_not_become_an_empty_catalogue() {
        assert!(model::catalog(br#"{}"#).is_err());
        assert!(model::catalog(&snapshot(&["a", "a"], 1)).is_err());
        assert!(model::catalog(&snapshot(&[""], 1)).is_err());
        let mut d = Dashboard::default();
        assert!(
            !d.render(20, 100, 0)
                .join("\n")
                .contains("No sessions listed")
        );
        d.permission = "WebAccess denied; grant permission and reload the plugin".into();
        assert!(d.render(20, 120, 0).join("\n").contains("WebAccess denied"));
    }

    #[test]
    fn reported_nested_usage_and_wrapped_paths_remain_inspectable() {
        let mut d = Dashboard::default();
        let (serial, _) = d.request(Endpoint::Catalog, 0, true).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&snapshot(&["a"], 1)).unwrap();
        value["sessions"][0]["usage"] = json!({
            "tokens": {"totalTokens": 42, "inputTokens": null, "outputTokens": 10},
            "estimatedCost": {"amount": 1.5, "currency": "USD"},
            "actualSessionCost": {"amount": 2.0, "currency": "USD"}
        });
        d.response(serial, 200, &serde_json::to_vec(&value).unwrap(), 0);
        let lines = d.detail_lines(d.session().unwrap()).join("\n");
        assert!(lines.contains("Tokens: 42 | Input: unknown | Output: 10"));
        assert!(lines.contains("Estimated cost: 1.5 USD | Actual session cost: 2 USD"));
        let path = "/a/long/界/path/which/must/remain/inspectable";
        let wrapped = wrap_detail(path, 9);
        assert_eq!(wrapped.concat(), path);
        assert!(
            wrapped
                .iter()
                .all(|s| unicode_width::UnicodeWidthStr::width(s.as_str()) <= 9)
        );
    }

    #[test]
    fn a_fresh_catalogue_cannot_hide_a_failed_status_refresh() {
        let mut d = Dashboard::default();
        let (serial, _) = d.request(Endpoint::Status, 0, true).unwrap();
        d.response(
            serial,
            200,
            br#"{"name":"hive","version":"1","sessions":1}"#,
            0,
        );
        let (serial, _) = d.request(Endpoint::Status, 20, true).unwrap();
        d.response(serial, 503, b"", 20);
        receive(&mut d, &["a"], 1, 20);
        assert!(d.freshness(Endpoint::Catalog, 20).contains("Fresh"));
        assert!(d.freshness(Endpoint::Status, 20).contains("STALE"));
        assert_eq!(d.status.as_ref().unwrap().version, "1");
    }
}
