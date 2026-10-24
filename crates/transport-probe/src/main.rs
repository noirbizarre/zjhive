//! Deliberately small host probe: native unit tests cannot establish WASI networking.
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct Probe {
    base: String,
    ticks: u64,
    observations: Vec<String>,
}

register_plugin!(Probe);

impl ZellijPlugin for Probe {
    fn load(&mut self, config: BTreeMap<String, String>) {
        self.base = config.get("url").cloned().unwrap_or_default();
        // An explicit address avoids conflating DNS failure with socket availability.
        let socket = std::net::TcpStream::connect("127.0.0.1:18769");
        self.observations.push(format!("socket: {socket:?}"));
        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::WebRequestResult,
            EventType::Timer,
        ]);
        request_permission(&[PermissionType::WebAccess]);
        set_timeout(1.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                for path in ["finite", "sse"] {
                    web_request(
                        format!("{}/{path}", self.base),
                        HttpVerb::Get,
                        BTreeMap::new(),
                        vec![],
                        BTreeMap::from([("kind".into(), path.into())]),
                    );
                }
            }
            Event::WebRequestResult(status, _, body, context) => {
                if context.get("kind").is_some_and(|k| k != "report") {
                    let kind = context.get("kind").unwrap();
                    let observation = format!(
                        "{kind}: status={status} ticks={} body={}",
                        self.ticks,
                        String::from_utf8_lossy(&body)
                    );
                    self.observations.push(observation);
                    web_request(
                        format!("{}/report", self.base),
                        HttpVerb::Post,
                        BTreeMap::new(),
                        self.observations.join("\n").into_bytes(),
                        BTreeMap::from([("kind".into(), "report".into())]),
                    );
                }
            }
            Event::Timer(_) => {
                self.ticks += 1;
                set_timeout(1.0);
            }
            _ => {}
        }
        true
    }

    fn render(&mut self, _: usize, _: usize) {
        println!("ZJHive transport probe: {} responsive ticks", self.ticks);
        for line in &self.observations {
            println!("{line}");
        }
    }
}
