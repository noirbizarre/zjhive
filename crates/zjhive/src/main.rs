//! A finite-HTTP, read-only Hive dashboard for Zellij.

mod dashboard;
mod model;

use dashboard::{Dashboard, Endpoint};
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct ZjHive {
    dashboard: Dashboard,
    permitted: bool,
    tick: u64,
    page: usize,
}

register_plugin!(ZjHive);

impl ZjHive {
    fn refresh(&mut self, manual: bool) {
        if !self.permitted {
            return;
        }
        for endpoint in Endpoint::ALL {
            if let Some((serial, url)) = self.dashboard.request(endpoint, self.tick, manual) {
                web_request(
                    &url,
                    HttpVerb::Get,
                    BTreeMap::new(),
                    vec![],
                    BTreeMap::from([("serial".into(), serial.to_string())]),
                );
            }
        }
    }
}

impl ZellijPlugin for ZjHive {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.dashboard = Dashboard::new(
            configuration
                .get("url")
                .map(String::as_str)
                .unwrap_or("http://127.0.0.1:8765"),
            configuration.contains_key("bridge"),
        );
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::WebRequestResult,
            EventType::Timer,
            EventType::Key,
        ]);
        request_permission(&[PermissionType::WebAccess]);
        set_timeout(1.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                self.permitted = result == PermissionStatus::Granted;
                self.dashboard.permission = if self.permitted {
                    "WebAccess granted".into()
                } else {
                    "WebAccess denied; grant permission and reload the plugin".into()
                };
                self.refresh(true);
            }
            Event::WebRequestResult(status, _, body, context) => {
                if let Some(serial) = context.get("serial").and_then(|s| s.parse().ok()) {
                    self.dashboard.response(serial, status, &body, self.tick);
                }
            }
            Event::Timer(_) => {
                self.tick += 1;
                self.dashboard.expire(self.tick);
                self.refresh(false);
                set_timeout(1.0);
            }
            Event::Key(key) => match key.bare_key {
                BareKey::Char('r') => self.refresh(true),
                BareKey::Down => self.dashboard.navigate(1),
                BareKey::Up => self.dashboard.navigate(-1),
                BareKey::PageDown => self.dashboard.navigate(self.page.max(1) as isize),
                BareKey::PageUp => self.dashboard.navigate(-(self.page.max(1) as isize)),
                BareKey::Home => self.dashboard.home(),
                BareKey::End => self.dashboard.end(),
                BareKey::Enter => self.dashboard.open_details(),
                BareKey::Esc => self.dashboard.close_details(),
                _ => {}
            },
            _ => return false,
        }
        true
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.page = rows.saturating_sub(7);
        for line in self.dashboard.render(rows, cols, self.tick) {
            println!("{line}");
        }
    }
}
