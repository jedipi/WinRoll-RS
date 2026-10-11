use self_update::UpdateConfig;
use self_update::{Release, ReleaseAsset};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread,
    time::Duration,
};

mod install_mode;
mod portable;

pub fn startup_status(status: &str) {
    static REPORTED: std::sync::Once = std::sync::Once::new();
    // Later runtime errors must not change a successful startup result.
    REPORTED.call_once(|| {
        if let Some(path) = std::env::var_os("WINROLL_UPDATE_STATUS") {
            let path = std::path::PathBuf::from(path);
            let temporary = path.with_extension("tmp");
            let _ =
                std::fs::write(&temporary, status).and_then(|()| std::fs::rename(temporary, path));
        }
    });
}

#[derive(Clone, Debug)]
pub struct Offer {
    pub version: String,
    release: Release,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Checking,
    Current,
    Available(String),
    Failed(String),
    Downloading { downloaded: u64, total: Option<u64> },
    Verifying,
    Staged(String),
    DownloadFailed(String),
    Recovering,
    Installing,
    InstallFailed(String),
    ReadyToExit,
}

pub struct StagedPackage {
    pub version: String,
    pub path: std::path::PathBuf,
    _directory: tempfile::TempDir,
}

enum Event {
    Checked(Result<Option<Offer>, String>),
    Progress(State),
    Downloaded(Result<StagedPackage, String>),
    Installed(Result<(), String>),
}

#[derive(Clone)]
pub struct Progress {
    sender: SyncSender<Event>,
    canceled: Arc<AtomicBool>,
}

impl Progress {
    pub fn report(&self, downloaded: u64, total: Option<u64>) {
        let _ = self
            .sender
            .try_send(Event::Progress(State::Downloading { downloaded, total }));
    }

    fn check(&self) -> io::Result<()> {
        if self.canceled.load(Ordering::Relaxed) {
            // Write::write_all retries Interrupted, so cancellation must be a terminal error.
            Err(io::Error::other("Update canceled."))
        } else {
            Ok(())
        }
    }
}

pub struct Checker {
    pub state: State,
    available: Option<Offer>,
    pending: Option<Receiver<Event>>,
    canceled: Option<Arc<AtomicBool>>,
    pub staged: Option<StagedPackage>,
}

impl Checker {
    pub const fn new() -> Self {
        Self {
            state: State::Idle,
            available: None,
            pending: None,
            canceled: None,
            staged: None,
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
        service: impl FnOnce() -> Result<Option<Offer>, String> + Send + 'static,
    ) {
        self.cancel();
        let (sender, receiver) = mpsc::sync_channel(1);
        self.pending = Some(receiver);
        self.state = State::Checking;
        if let Err(error) = thread::Builder::new()
            .name("update-check".into())
            .spawn(move || {
                let _ = sender.send(Event::Checked(service()));
            })
        {
            self.pending = None;
            self.state = State::Failed(error.to_string());
        }
    }

    pub fn busy(&self) -> bool {
        matches!(
            self.state,
            State::Checking
                | State::Downloading { .. }
                | State::Verifying
                | State::Recovering
                | State::Installing
        )
    }

    pub fn update_now(&mut self) {
        self.update_with(|offer, progress| {
            let installed = install_mode::installed()?;
            stage(offer, installed, progress, download_package)
        });
    }

    pub fn update_with(
        &mut self,
        service: impl FnOnce(Offer, Progress) -> Result<StagedPackage, String> + Send + 'static,
    ) {
        if !matches!(
            self.state,
            State::Available(_) | State::DownloadFailed(_) | State::InstallFailed(_)
        ) {
            return;
        }
        let Some(offer) = self.available.clone() else {
            return;
        };
        self.cancel();
        let (sender, receiver) = mpsc::sync_channel(1);
        let canceled = Arc::new(AtomicBool::new(false));
        let progress = Progress {
            sender: sender.clone(),
            canceled: canceled.clone(),
        };
        self.pending = Some(receiver);
        self.canceled = Some(canceled);
        self.state = State::Downloading {
            downloaded: 0,
            total: None,
        };
        if let Err(error) = thread::Builder::new()
            .name("update-download".into())
            .spawn(move || {
                let _ = sender.send(Event::Downloaded(service(offer, progress)));
            })
        {
            self.pending = None;
            self.state = State::DownloadFailed(error.to_string());
        }
    }

    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let event = match receiver.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                if matches!(self.state, State::Checking) {
                    Event::Checked(Err("Update check stopped unexpectedly.".into()))
                } else if self.state == State::Installing {
                    Event::Installed(Err("Update installation stopped unexpectedly.".into()))
                } else {
                    Event::Downloaded(Err("Update download stopped unexpectedly.".into()))
                }
            }
        };
        if let Event::Progress(state) = event {
            self.state = state;
            return true;
        }
        self.pending = None;
        self.canceled = None;
        self.state = match event {
            Event::Checked(Ok(offer)) => {
                self.available = offer.clone();
                offer.map_or(State::Current, |offer| State::Available(offer.version))
            }
            Event::Checked(Err(error)) => State::Failed(error),
            Event::Downloaded(Ok(package)) => {
                let state = State::Staged(package.version.clone());
                self.staged = Some(package);
                state
            }
            Event::Downloaded(Err(error)) => State::DownloadFailed(error),
            Event::Installed(Ok(())) => State::ReadyToExit,
            Event::Installed(Err(error)) => State::InstallFailed(error),
            Event::Progress(_) => unreachable!(),
        };
        true
    }

    pub fn cancel(&mut self) {
        if matches!(self.state, State::Installing | State::ReadyToExit) {
            return;
        }
        if let Some(canceled) = self.canceled.take() {
            canceled.store(true, Ordering::Relaxed);
        }
        // Dropping this receiver makes late results unable to affect a new dialog or retry.
        self.pending = None;
        self.staged = None;
        self.state = self
            .available
            .as_ref()
            .map_or(State::Idle, |offer| State::Available(offer.version.clone()));
    }

    pub fn recover(&mut self) -> bool {
        if !matches!(self.state, State::Staged(_)) {
            return false;
        }
        if self
            .staged
            .as_ref()
            .is_none_or(|package| package.path.extension().is_none_or(|ext| ext != "zip"))
        {
            self.staged = None;
            self.state = State::InstallFailed("Installed updates are not supported yet.".into());
            return false;
        }
        self.state = State::Recovering;
        true
    }

    pub fn recovered(&mut self) {
        self.install_with(portable::prepare);
    }

    pub fn install_with(
        &mut self,
        service: impl FnOnce(StagedPackage) -> Result<(), String> + Send + 'static,
    ) {
        if self.state != State::Recovering {
            return;
        }
        let Some(package) = self.staged.take() else {
            return;
        };
        let (sender, receiver) = mpsc::sync_channel(1);
        self.pending = Some(receiver);
        self.state = State::Installing;
        if let Err(error) = thread::Builder::new()
            .name("update-install".into())
            .spawn(move || {
                let _ = sender.send(Event::Installed(service(package)));
            })
        {
            self.pending = None;
            self.state = State::InstallFailed(error.to_string());
        }
    }
}

fn download_package(
    asset: &ReleaseAsset,
    writer: &mut dyn Write,
    progress: &Progress,
) -> Result<(), String> {
    let progress = progress.clone();
    self_update::Download::from_url(asset.download_url())
        .timeout(Duration::from_secs(30))
        .request_header("Accept", "application/octet-stream")
        .progress_callback(move |bytes, total| progress.report(bytes, total))
        .download_to(writer)
        .map_err(|error| error.to_string())
}

fn newer_stable(current: &str, version: &str) -> Result<Option<String>, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let newer = self_update::version::bump_is_greater(current, version)
        .map_err(|error| error.to_string())?;
    let prerelease = version.split('+').next().unwrap_or(version).contains('-');
    Ok((newer && !prerelease).then(|| version.to_owned()))
}

pub fn check() -> Result<Option<Offer>, String> {
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

fn discover(updater: &self_update::backends::github::Update) -> Result<Option<Offer>, String> {
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
        Some(release) => Ok(
            newer_stable(updater.current_version(), release.version())?.map(|version| Offer {
                version,
                release: release.clone(),
            }),
        ),
        None => Ok(None),
    }
}

fn stage(
    offer: Offer,
    installed: bool,
    progress: Progress,
    download: impl FnOnce(&ReleaseAsset, &mut dyn Write, &Progress) -> Result<(), String>,
) -> Result<StagedPackage, String> {
    progress.check().map_err(|e| e.to_string())?;
    let name = if installed {
        format!("winroll-{}-x64-setup.exe", offer.version)
    } else {
        format!("winroll-{}-x64.zip", offer.version)
    };
    let portable_name = format!("winroll-{}-x64-portable.zip", offer.version);
    let asset = offer
        .release
        .assets()
        .iter()
        .find(|asset| asset.name() == name || (!installed && asset.name() == portable_name))
        .ok_or("No matching update package was found.")?;
    let digest = asset
        .digest()
        .ok_or("Update package has no SHA-256 digest.")?;
    let expected = digest
        .strip_prefix("sha256:")
        .filter(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("Update package has an invalid SHA-256 digest.")?;
    let directory = tempfile::Builder::new()
        .prefix("winroll-update-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let path = directory.path().join(asset.name());
    let mut writer = PackageWriter {
        file: File::create(&path).map_err(|e| e.to_string())?,
        hash: Sha256::new(),
        progress,
    };
    let progress = writer.progress.clone();
    download(asset, &mut writer, &progress)?;
    writer.progress.check().map_err(|e| e.to_string())?;
    writer
        .progress
        .sender
        .send(Event::Progress(State::Verifying))
        .map_err(|_| "Update canceled.")?;
    writer.file.sync_all().map_err(|e| e.to_string())?;
    let actual = format!("{:x}", writer.hash.finalize());
    if !actual.eq_ignore_ascii_case(expected) {
        return Err("Update package SHA-256 does not match.".into());
    }
    writer.progress.check().map_err(|e| e.to_string())?;
    // Close the writer before handing the verified package to a later installation stage.
    drop(writer.file);
    Ok(StagedPackage {
        version: offer.version,
        path,
        _directory: directory,
    })
}

struct PackageWriter {
    file: File,
    hash: Sha256,
    progress: Progress,
}

impl Write for PackageWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.progress.check()?;
        let written = self.file.write(bytes)?;
        self.hash.update(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.progress.check()?;
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const HELLO_DIGEST: &str =
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn offer(version: &str, digest: Option<&str>) -> Offer {
        let assets = [
            format!("winroll-{version}-x64.zip"),
            format!("winroll-{version}-x64-setup.exe"),
        ];
        let mut release = Release::builder();
        release.version(version);
        for name in assets {
            let mut asset = ReleaseAsset::new(&name, format!("https://fixture/{name}"));
            if let Some(digest) = digest {
                asset = asset.with_digest(digest);
            }
            release.asset(asset);
        }
        Offer {
            version: version.into(),
            release: release.build().unwrap(),
        }
    }

    fn settle(checker: &mut Checker) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while checker.busy() {
            checker.poll();
            assert!(Instant::now() < deadline, "Update did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn installation_requires_verified_portable_package_and_recovery() {
        let mut checker = Checker::new();
        checker.install_with(|_| panic!("Cannot install before verification"));
        for installed in [true, false] {
            checker.start(|| Ok(Some(offer("1.1.0", Some(HELLO_DIGEST)))));
            settle(&mut checker);
            checker.update_with(move |offer, progress| {
                stage(offer, installed, progress, |_, writer, _| {
                    writer.write_all(b"hello").map_err(|e| e.to_string())
                })
            });
            settle(&mut checker);
            checker.install_with(|_| panic!("Cannot install before recovery"));
            assert_eq!(checker.recover(), !installed);
            if installed {
                assert!(matches!(checker.state, State::InstallFailed(_)));
                continue;
            }
            let path = checker.staged.as_ref().unwrap().path.clone();
            checker.cancel();
            checker.install_with(|_| panic!("Canceled recovery cannot install or exit"));
            assert!(!path.exists());
            assert_eq!(checker.state, State::Available("1.1.0".into()));
        }
        checker.update_with(|offer, progress| {
            stage(offer, false, progress, |_, writer, _| {
                writer.write_all(b"hello").map_err(|e| e.to_string())
            })
        });
        settle(&mut checker);
        assert!(checker.recover());
        let (ready, observed) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        checker.install_with(move |package| {
            assert_eq!(std::fs::read(package.path).unwrap(), b"hello");
            ready.send(()).unwrap();
            wait.recv().unwrap();
            Ok(())
        });
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
        checker.cancel();
        assert_eq!(checker.state, State::Installing);
        assert!(!checker.poll(), "Installation does not block UI polling");
        release.send(()).unwrap();
        settle(&mut checker);
        assert_eq!(checker.state, State::ReadyToExit);
    }

    #[test]
    fn installation_failure_allows_retry_without_exit() {
        let mut checker = Checker::new();
        checker.start(|| Ok(Some(offer("1.1.0", Some(HELLO_DIGEST)))));
        settle(&mut checker);
        for succeeds in [false, true] {
            checker.update_with(|offer, progress| {
                stage(offer, false, progress, |_, writer, _| {
                    writer.write_all(b"hello").map_err(|e| e.to_string())
                })
            });
            settle(&mut checker);
            assert!(checker.recover());
            checker.install_with(move |_| {
                if succeeds {
                    Ok(())
                } else {
                    Err("locked".into())
                }
            });
            settle(&mut checker);
            assert_eq!(
                checker.state,
                if succeeds {
                    State::ReadyToExit
                } else {
                    State::InstallFailed("locked".into())
                }
            );
        }
    }

    #[test]
    fn consent_selects_and_verifies_the_matching_package() {
        for installed in [false, true] {
            let mut checker = Checker::new();
            checker.update_with(|_, _| panic!("No offer, no consent"));
            checker.start(|| Ok(Some(offer("1.1.0", Some(HELLO_DIGEST)))));
            settle(&mut checker);
            assert!(checker.staged.is_none(), "Checking must not download");
            checker.update_with(move |offer, progress| {
                stage(offer, installed, progress, |asset, writer, progress| {
                    assert!(
                        asset
                            .name()
                            .ends_with(if installed { "-setup.exe" } else { ".zip" })
                    );
                    progress.report(2, Some(5));
                    writer.write_all(b"hello").map_err(|e| e.to_string())
                })
            });
            settle(&mut checker);
            assert_eq!(checker.state, State::Staged("1.1.0".into()));
            let package = checker.staged.as_ref().unwrap();
            let path = package.path.clone();
            assert_eq!(std::fs::read(&path).unwrap(), b"hello");
            checker.cancel();
            assert!(!path.exists(), "Canceled staging must be cleaned up");
        }
    }

    #[test]
    fn published_portable_asset_can_be_staged() {
        let mut checker = Checker::new();
        checker.start(|| {
            Ok(Some(Offer {
                version: "1.0.0".into(),
                release: Release::builder()
                    .version("1.0.0")
                    .asset(
                        ReleaseAsset::new(
                            "winroll-1.0.0-x64-portable.zip",
                            "https://fixture/portable",
                        )
                        .with_digest(HELLO_DIGEST),
                    )
                    .asset(
                        ReleaseAsset::new(
                            "winroll-1.0.0-x64-setup.exe",
                            "https://fixture/installer",
                        )
                        .with_digest(HELLO_DIGEST),
                    )
                    .build()
                    .unwrap(),
            }))
        });
        settle(&mut checker);
        checker.update_with(|offer, progress| {
            stage(offer, false, progress, |asset, writer, _| {
                assert_eq!(asset.name(), "winroll-1.0.0-x64-portable.zip");
                writer
                    .write_all(b"hello")
                    .map_err(|error| error.to_string())
            })
        });
        settle(&mut checker);
        assert_eq!(checker.state, State::Staged("1.0.0".into()));
        assert_eq!(
            checker.staged.as_ref().unwrap().path.file_name().unwrap(),
            "winroll-1.0.0-x64-portable.zip"
        );
    }

    #[test]
    fn rejected_and_interrupted_downloads_allow_retry() {
        for (digest, interrupted, expected) in [
            (None, false, "Update package has no SHA-256 digest."),
            (
                Some("sha256:123"),
                false,
                "Update package has an invalid SHA-256 digest.",
            ),
            (
                Some("sha512:abc"),
                false,
                "Update package has an invalid SHA-256 digest.",
            ),
            (
                Some("sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"),
                false,
                "Update package has an invalid SHA-256 digest.",
            ),
            (
                Some("sha256:0000000000000000000000000000000000000000000000000000000000000000"),
                false,
                "Update package SHA-256 does not match.",
            ),
            (Some(HELLO_DIGEST), true, "connection lost"),
        ] {
            let mut checker = Checker::new();
            checker.start(move || Ok(Some(offer("1.1.0", digest))));
            settle(&mut checker);
            checker.update_with(move |offer, progress| {
                stage(offer, false, progress, |_, writer, _| {
                    assert!(
                        digest.is_some_and(|digest| digest.len() == 71),
                        "Invalid metadata must not request bytes"
                    );
                    writer.write_all(b"hello").unwrap();
                    if interrupted {
                        Err("connection lost".into())
                    } else {
                        Ok(())
                    }
                })
            });
            settle(&mut checker);
            assert_eq!(checker.state, State::DownloadFailed(expected.into()));
            assert!(checker.staged.is_none());
            checker.update_with(|mut offer, progress| {
                offer.release = self::offer("1.1.0", Some(HELLO_DIGEST)).release;
                stage(offer, false, progress, |_, writer, _| {
                    writer.write_all(b"hello").map_err(|e| e.to_string())
                })
            });
            settle(&mut checker);
            assert_eq!(checker.state, State::Staged("1.1.0".into()));
        }
    }

    #[test]
    fn missing_package_and_network_failure_cannot_stage() {
        for missing in [true, false] {
            let mut checker = Checker::new();
            checker.start(move || {
                let mut offer = offer("1.1.0", Some(HELLO_DIGEST));
                if missing {
                    offer.release = Release::builder().version("1.1.0").build().unwrap();
                }
                Ok(Some(offer))
            });
            settle(&mut checker);
            checker.update_with(move |offer, progress| {
                stage(offer, false, progress, |_, _, _| {
                    assert!(!missing);
                    Err("offline".into())
                })
            });
            settle(&mut checker);
            assert_eq!(
                checker.state,
                State::DownloadFailed(
                    if missing {
                        "No matching update package was found."
                    } else {
                        "offline"
                    }
                    .into()
                )
            );
            assert!(checker.staged.is_none());
        }
    }

    #[test]
    fn progress_is_responsive_and_canceled_download_cannot_resume() {
        for total in [None, Some(5)] {
            let mut checker = Checker::new();
            checker.start(|| Ok(Some(offer("1.1.0", Some(HELLO_DIGEST)))));
            settle(&mut checker);
            let (ready, observed) = mpsc::channel();
            let (release, wait) = mpsc::channel();
            let (completed, done) = mpsc::channel();
            checker.update_with(move |offer, progress| {
                stage(offer, false, progress, move |_, writer, progress| {
                    writer.write_all(b"he").unwrap();
                    progress.report(2, total);
                    ready.send(()).unwrap();
                    wait.recv().unwrap();
                    let result = writer.write_all(b"llo");
                    assert!(result.is_err(), "Canceled writer must reject more bytes");
                    completed.send(()).unwrap();
                    result.map_err(|e| e.to_string())
                })
            });
            observed.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(checker.poll());
            assert_eq!(
                checker.state,
                State::Downloading {
                    downloaded: 2,
                    total
                }
            );
            assert!(!checker.poll(), "Blocked download cannot block UI polling");
            checker.cancel();
            checker.update_with(|offer, progress| {
                stage(offer, false, progress, |_, writer, _| {
                    writer.write_all(b"hello").map_err(|e| e.to_string())
                })
            });
            settle(&mut checker);
            release.send(()).unwrap();
            done.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(!checker.poll());
            assert_eq!(checker.state, State::Staged("1.1.0".into()));
        }
    }

    #[test]
    fn late_verified_package_is_discarded_after_cancellation() {
        let mut checker = Checker::new();
        checker.start(|| Ok(Some(offer("1.1.0", Some(HELLO_DIGEST)))));
        settle(&mut checker);
        let (ready, observed) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        checker.update_with(move |offer, progress| {
            let package = stage(offer, false, progress, |_, writer, _| {
                writer.write_all(b"hello").map_err(|e| e.to_string())
            })?;
            ready.send(package.path.clone()).unwrap();
            wait.recv().unwrap();
            Ok(package)
        });
        finish(&mut checker); // Consume the verification event so the worker can complete staging.
        let path = observed.recv_timeout(Duration::from_secs(2)).unwrap();
        checker.cancel();
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while path.exists() {
            assert!(Instant::now() < deadline, "Late package was not deleted");
            thread::yield_now();
        }
        assert!(!checker.poll());
        assert!(checker.staged.is_none());
        assert_eq!(checker.state, State::Available("1.1.0".into()));
    }

    #[test]
    fn library_download_stages_bytes_from_a_deterministic_server() {
        use std::{io::Read, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/package", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0u8; 4096];
            assert!(socket.read(&mut request).unwrap() > 0);
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                )
                .unwrap();
        });
        let mut checker = Checker::new();
        checker.start(move || {
            Ok(Some(Offer {
                version: "1.1.0".into(),
                release: Release::builder()
                    .version("1.1.0")
                    .asset(
                        ReleaseAsset::new("winroll-1.1.0-x64.zip", url).with_digest(HELLO_DIGEST),
                    )
                    .build()
                    .unwrap(),
            }))
        });
        settle(&mut checker);
        checker.update_with(|offer, progress| stage(offer, false, progress, download_package));
        settle(&mut checker);
        server.join().unwrap();
        assert_eq!(checker.state, State::Staged("1.1.0".into()));
        assert_eq!(
            std::fs::read(&checker.staged.as_ref().unwrap().path).unwrap(),
            b"hello"
        );
    }

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
        checker.start(|| Ok(Some(offer("1.1.0", None))));
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
            Ok(Some(offer("9.0.0", None)))
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
                        r#"{{"tag_name":"{tag}","created_at":"2026-10-01T00:00:00Z","assets":[{{"name":"winroll-1.1.0-x64.zip","url":"https://fixture/package","digest":"{HELLO_DIGEST}"}}]}}"#
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
                if let Some(offer) = result.as_ref().unwrap() {
                    assert_eq!(offer.release.assets()[0].digest(), Some(HELLO_DIGEST));
                }
                assert_eq!(
                    result.map(|offer| offer.map(|offer| offer.version)),
                    Ok(expected.map(str::to_owned))
                );
            } else {
                assert!(
                    result.is_err(),
                    "Missing repository must not be reported current"
                );
            }
        }
    }
}
