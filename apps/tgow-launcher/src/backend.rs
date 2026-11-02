use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tgow_core::catalog::{self, App, Format, ReleaseState, Repository};
use tgow_net::{Client, Error, Fetch};
use tgow_platform::storage;
use tgow_ui::shell::CatalogStatus;

use crate::media::{self, Slot};

const MANIFEST_LIMIT: u64 = 256 * 1024;
const MEDIA_LIMIT: u64 = 4 * 1024 * 1024;
const SIGNATURE_LIMIT: u64 = 1024;
const OFFLINE_RETRY: Duration = Duration::from_secs(120);

pub(crate) enum Command {
    Refresh,
    Interval(Duration),
}

pub(crate) enum Event {
    Apps(Vec<App>),
    Media(String, Slot, egui::ColorImage),
    Launcher(Box<App>),
    Status(CatalogStatus),
    Checked,
}

pub(crate) struct Backend {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

impl Backend {
    pub(crate) fn start(
        ctx: egui::Context,
        cache_file: Option<PathBuf>,
        interval: Duration,
    ) -> Self {
        let (commands, inbox) = mpsc::channel();
        let (outbox, events) = mpsc::channel();
        let mut cache: Cache = cache_file
            .as_deref()
            .and_then(storage::read_text)
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        cache.repositories.retain(Repository::is_listed);
        let worker = Worker {
            client: Client::new(),
            cache,
            cache_file,
            outbox,
            ctx,
            interval,
            paused_until: None,
            shown: Vec::new(),
        };
        let _ = thread::Builder::new()
            .name("catalog".to_owned())
            .spawn(move || worker.run(&inbox));
        Self { commands, events }
    }

    pub(crate) fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub(crate) fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    etag: Option<String>,
    repositories: Vec<Repository>,
    releases: BTreeMap<String, Release>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Release {
    tag: Option<String>,
    legacy: bool,
    manifest: Option<String>,
    signature: Option<String>,
}

struct Worker {
    client: Client,
    cache: Cache,
    cache_file: Option<PathBuf>,
    outbox: Sender<Event>,
    ctx: egui::Context,
    interval: Duration,
    paused_until: Option<Instant>,
    shown: Vec<String>,
}

enum Outcome {
    Done,
    Offline,
    Limited(Duration),
}

impl Worker {
    fn run(mut self, inbox: &Receiver<Command>) {
        if !self.cache.repositories.is_empty() {
            self.publish_apps();
        }
        let mut wait = self.refresh();
        loop {
            match inbox.recv_timeout(wait) {
                Ok(Command::Refresh) | Err(RecvTimeoutError::Timeout) => wait = self.refresh(),
                Ok(Command::Interval(interval)) => {
                    self.interval = interval;
                    wait = interval;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn emit(&self, event: Event) {
        let _ = self.outbox.send(event);
        self.ctx.request_repaint();
    }

    fn refresh(&mut self) -> Duration {
        if let Some(until) = self.paused_until {
            let now = Instant::now();
            if until > now {
                let remaining = until - now;
                self.emit(Event::Status(CatalogStatus::Limited {
                    minutes: remaining.as_secs().div_ceil(60),
                }));
                return remaining;
            }
            self.paused_until = None;
        }
        self.emit(Event::Status(CatalogStatus::Loading));
        let outcome = self.check();
        self.publish_apps();
        self.save();
        self.publish_media();
        match outcome {
            Outcome::Done => {
                self.emit(Event::Status(CatalogStatus::Ready));
                self.emit(Event::Checked);
                self.interval
            }
            Outcome::Offline => {
                self.emit(Event::Status(CatalogStatus::Offline));
                OFFLINE_RETRY.min(self.interval)
            }
            Outcome::Limited(wait) => {
                self.paused_until = Some(Instant::now() + wait);
                self.emit(Event::Status(CatalogStatus::Limited {
                    minutes: wait.as_secs().div_ceil(60),
                }));
                wait
            }
        }
    }

    fn check(&mut self) -> Outcome {
        match self.client.repositories(self.cache.etag.as_deref()) {
            Ok(Fetch::Fresh { body, etag }) => {
                if let Ok(listed) = catalog::listed_repositories(&body) {
                    self.cache.repositories = listed;
                    self.cache.etag = etag;
                }
            }
            Ok(Fetch::NotModified) => {}
            Err(Error::RateLimited(wait)) => return Outcome::Limited(wait),
            Err(_) => return Outcome::Offline,
        }
        let mut names: Vec<String> = self
            .cache
            .repositories
            .iter()
            .map(|repository| repository.name.clone())
            .collect();
        if !names.iter().any(|name| name == catalog::LAUNCHER_REPO) {
            names.push(catalog::LAUNCHER_REPO.to_owned());
        }
        self.cache.releases.retain(|name, _| names.contains(name));
        for name in names {
            match self.client.latest_tag(&name) {
                Ok(tag) => self.update_release(&name, tag),
                Err(Error::RateLimited(wait)) => return Outcome::Limited(wait),
                Err(_) => return Outcome::Offline,
            }
        }
        Outcome::Done
    }

    fn update_release(&mut self, name: &str, tag: Option<String>) {
        let known = self.cache.releases.get(name);
        if known.is_some_and(|release| release.tag == tag && release.manifest.is_some()) {
            return;
        }
        let Some(tag) = tag else {
            self.cache
                .releases
                .insert(name.to_owned(), Release::default());
            return;
        };
        let mut release = Release {
            tag: Some(tag.clone()),
            ..Release::default()
        };
        for format in [Format::Current, Format::Legacy] {
            let url = catalog::download_url(name, &tag, format.file());
            let Ok(manifest) = self.client.fetch(&url, MANIFEST_LIMIT) else {
                continue;
            };
            let signature = self
                .client
                .fetch(
                    &format!("{url}{}", catalog::SIGNATURE_SUFFIX),
                    SIGNATURE_LIMIT,
                )
                .ok();
            release.legacy = format == Format::Legacy;
            release.manifest = String::from_utf8(manifest).ok();
            release.signature = signature.and_then(|bytes| String::from_utf8(bytes).ok());
            break;
        }
        self.cache.releases.insert(name.to_owned(), release);
    }

    fn publish_apps(&self) {
        let launcher = launcher_repository();
        let release = self.cache.releases.get(catalog::LAUNCHER_REPO);
        let mut app = App::from_repository(&launcher);
        app.tag = release.and_then(|release| release.tag.clone());
        app.release = release.map_or(ReleaseState::Unknown, |release| state(&launcher, release));
        self.emit(Event::Launcher(Box::new(app)));
        let apps = self
            .cache
            .repositories
            .iter()
            .filter(|repository| repository.name != catalog::LAUNCHER_REPO)
            .map(|repository| {
                let mut app = App::from_repository(repository);
                let release = self.cache.releases.get(&repository.name);
                app.tag = release.and_then(|release| release.tag.clone());
                app.release =
                    release.map_or(ReleaseState::Unknown, |release| state(repository, release));
                app
            })
            .collect();
        self.emit(Event::Apps(apps));
    }

    fn publish_media(&mut self) {
        let Some(folder) = self
            .cache_file
            .as_ref()
            .and_then(|file| file.parent())
            .map(|cache| cache.join("media"))
        else {
            return;
        };
        let wanted: Vec<(String, String, String, Slot)> = self
            .cache
            .repositories
            .iter()
            .filter_map(|repository| {
                let release = self.cache.releases.get(&repository.name)?;
                let tag = release.tag.clone()?;
                let ReleaseState::Verified(manifest) = state(repository, release) else {
                    return None;
                };
                Some(
                    [
                        (manifest.icon.clone(), Slot::Icon),
                        (manifest.hero.clone(), Slot::Hero),
                    ]
                    .into_iter()
                    .filter_map(|(file, slot)| {
                        Some((repository.name.clone(), tag.clone(), file?, slot))
                    })
                    .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();
        for (repository, tag, file, slot) in wanted {
            let key = format!("{repository}/{tag}/{file}");
            if self.shown.contains(&key) {
                continue;
            }
            let path = folder.join(&repository).join(&tag).join(&file);
            let bytes = storage_bytes(&path).or_else(|| {
                let url = catalog::download_url(&repository, &tag, &file);
                let bytes = self.client.fetch(&url, MEDIA_LIMIT).ok()?;
                let _ = storage::write_atomic(&path, &bytes);
                Some(bytes)
            });
            if let Some(image) = bytes.as_deref().and_then(media::decode) {
                self.shown.push(key);
                self.emit(Event::Media(repository.to_ascii_lowercase(), slot, image));
            }
        }
    }

    fn save(&self) {
        if let (Some(file), Ok(json)) = (&self.cache_file, serde_json::to_string(&self.cache)) {
            let _ = storage::write_atomic(file, json.as_bytes());
        }
    }
}

fn storage_bytes(path: &std::path::Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

fn launcher_repository() -> Repository {
    Repository {
        name: catalog::LAUNCHER_REPO.to_owned(),
        full_name: format!("{}/{}", catalog::OWNER, catalog::LAUNCHER_REPO),
        description: None,
        topics: vec![catalog::TOPIC.to_owned()],
        archived: false,
        fork: false,
        private: false,
        html_url: format!(
            "https://github.com/{}/{}",
            catalog::OWNER,
            catalog::LAUNCHER_REPO
        ),
        pushed_at: None,
    }
}

fn state(repository: &Repository, release: &Release) -> ReleaseState {
    let Some(tag) = &release.tag else {
        return ReleaseState::NoRelease;
    };
    match (&release.manifest, &release.signature) {
        (Some(manifest), Some(signature)) => {
            let format = if release.legacy {
                Format::Legacy
            } else {
                Format::Current
            };
            catalog::evaluate(repository, tag, format, manifest.as_bytes(), signature)
        }
        _ => ReleaseState::Unsigned,
    }
}
