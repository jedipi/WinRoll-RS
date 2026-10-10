use self_update::UpdateConfig;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    Current,
    Available(String),
    Failed(String),
}

pub struct Checker {
    pub state: State,
    available: Option<String>,
    pending: Option<Receiver<Result<Option<String>, String>>>,
}

impl Checker {
    pub const fn new() -> Self {
        Self {
            state: State::Idle,
            available: None,
            pending: None,
        }
    }

    pub fn menu_label(&self) -> &'static str {
        if self.available.is_some() {
            "&Update available..."
        } else {
            "&Check for update..."
        }
    }

    pub fn start(
        &mut self,
        service: impl FnOnce() -> Result<Option<String>, String> + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.state = State::Checking;
        if let Err(error) = thread::Builder::new()
            .name("update-check".into())
            .spawn(move || {
                let _ = sender.send(service());
            })
        {
            self.pending = None;
            self.state = State::Failed(error.to_string());
        }
    }

    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("Update check stopped unexpectedly.".into()),
        };
        self.pending = None;
        self.state = match result {
            Ok(version) => {
                self.available = version.clone();
                version.map_or(State::Current, State::Available)
            }
            Err(error) => State::Failed(error),
        };
        true
    }

    pub fn cancel(&mut self) {
        // Dropping this receiver makes late results unable to affect a new dialog or retry.
        self.pending = None;
        self.state = self.available.clone().map_or(State::Idle, State::Available);
    }
}

fn newer_stable(current: &str, version: &str) -> Result<Option<String>, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let newer = self_update::version::bump_is_greater(current, version)
        .map_err(|error| error.to_string())?;
    let prerelease = version.split('+').next().unwrap_or(version).contains('-');
    Ok((newer && !prerelease).then(|| version.to_owned()))
}

pub fn check() -> Result<Option<String>, String> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner("jedipi")
        .repo_name("WinRoll-RS")
        .bin_name("winroll.exe")
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .show_output(false)
        .show_download_progress(false)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?;
    discover(&updater)
}

fn discover(updater: &self_update::backends::github::Update) -> Result<Option<String>, String> {
    // GitHub's latest endpoint excludes draft/prerelease flags that the library's listing drops.
    let releases = match updater.get_latest_release() {
        Ok(releases) => releases,
        Err(error) if error.http_status() == Some(404) => {
            // Distinguish no stable release from an inaccessible repository before saying current.
            updater
                .get_newer_releases()
                .map_err(|error| error.to_string())?;
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    };
    match releases.latest() {
        Some(release) => newer_stable(updater.current_version(), release.version()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn finish(checker: &mut Checker) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !checker.poll() {
            assert!(Instant::now() < deadline, "Check did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn stable_release_selection() {
        for version in ["1.0.0", "0.9.0", "1.1.0-beta.1", "2.0.0-rc.1+build"] {
            assert_eq!(newer_stable("1.0.0", version), Ok(None));
        }
        for version in ["1.0.1", "1.1.0", "2.0.0", "1.1.0+build-with-hyphen"] {
            assert_eq!(newer_stable("1.0.0", version), Ok(Some(version.into())));
        }
        assert_eq!(newer_stable("1.0.0", "v1.1.0"), Ok(Some("1.1.0".into())));
        assert!(newer_stable("1.0.0", "nightly").is_err());
    }

    #[test]
    fn discovery_error_and_retry() {
        let mut checker = Checker::new();
        assert_eq!(checker.menu_label(), "&Check for update...");
        checker.start(|| Err("offline".into()));
        finish(&mut checker);
        assert_eq!(checker.state, State::Failed("offline".into()));
        checker.start(|| Ok(Some("1.1.0".into())));
        finish(&mut checker);
        assert_eq!(checker.state, State::Available("1.1.0".into()));
        assert_eq!(checker.menu_label(), "&Update available...");
        checker.start(|| Ok(None));
        finish(&mut checker);
        assert_eq!(checker.state, State::Current);
        assert_eq!(checker.menu_label(), "&Check for update...");
    }

    #[test]
    fn canceled_check_cannot_overwrite_retry() {
        let mut checker = Checker::new();
        let (release, wait) = mpsc::channel();
        let (completed, done) = mpsc::channel();
        checker.start(move || {
            wait.recv().unwrap();
            completed.send(()).unwrap();
            Ok(Some("9.0.0".into()))
        });
        assert_eq!(checker.state, State::Checking);
        assert!(!checker.poll(), "Blocked check must not block the caller");
        checker.cancel();
        checker.start(|| Ok(None));
        finish(&mut checker);
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!checker.poll());
        assert_eq!(checker.state, State::Current);
        assert_eq!(checker.menu_label(), "&Check for update...");
    }

    #[test]
    fn github_discovery_only_reads_release_metadata() {
        use self_update::http_client::{HeaderMap, HttpClient, HttpResponse};
        use std::{io::Cursor, sync::Arc};

        struct Fixture {
            tag: Option<&'static str>,
            exists: bool,
        }
        struct Response {
            headers: HeaderMap,
            body: String,
        }
        impl HttpResponse for Response {
            fn headers(&self) -> &HeaderMap {
                &self.headers
            }
            fn body(self: Box<Self>) -> Box<dyn std::io::Read> {
                Box::new(Cursor::new(self.body.into_bytes()))
            }
        }
        impl HttpClient for Fixture {
            fn get(
                &self,
                url: &str,
                _: &HeaderMap,
                _: Option<Duration>,
            ) -> self_update::Result<Box<dyn HttpResponse>> {
                let body = if url.ends_with("/releases/latest") {
                    let Some(tag) = self.tag else {
                        return Err(self_update::Error::http_status_error(404, url));
                    };
                    format!(
                        r#"{{"tag_name":"{tag}","created_at":"2026-10-01T00:00:00Z","assets":[]}}"#
                    )
                } else {
                    assert!(
                        url.contains("/releases?"),
                        "Discovery must never request a package: {url}"
                    );
                    if !self.exists {
                        return Err(self_update::Error::http_status_error(404, url));
                    }
                    "[]".into()
                };
                Ok(Box::new(Response {
                    headers: HeaderMap::new(),
                    body,
                }))
            }
        }
        for (tag, exists, expected) in [
            (Some("v1.1.0"), true, Some("1.1.0")),
            (Some("v1.0.0"), true, None),
            (Some("v0.9.0"), true, None),
            (Some("v2.0.0-beta"), true, None),
            (None, true, None),
            (None, false, None),
        ] {
            let updater = self_update::backends::github::Update::configure()
                .repo_owner("jedipi")
                .repo_name("WinRoll-RS")
                .bin_name("winroll.exe")
                .current_version("1.0.0")
                .http_client(Arc::new(Fixture { tag, exists }))
                .build()
                .unwrap();
            let result = discover(&updater);
            if exists {
                assert_eq!(result, Ok(expected.map(str::to_owned)));
            } else {
                assert!(
                    result.is_err(),
                    "Missing repository must not be reported current"
                );
            }
        }
    }
}
