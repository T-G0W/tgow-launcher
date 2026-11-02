use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tgow_core::catalog::App;
use tgow_core::install::{Installation, Ledger};
use tgow_core::platform::Platform;
use tgow_core::select::{self, Preference};
use tgow_core::settings::{Settings, UpdateMode};
use tgow_platform::{process, storage};
use tgow_ui::shell::Phase;

use crate::jobs::{Jobs, Order, Report, Task};

pub(crate) struct Library {
    ledger: Ledger,
    ledger_file: Option<PathBuf>,
    detected: BTreeMap<String, Installation>,
    pub(crate) installed: BTreeMap<String, Installation>,
    pub(crate) phases: BTreeMap<String, Phase>,
    pub(crate) failures: BTreeMap<String, String>,
    jobs: Option<Jobs>,
    parallel: Arc<AtomicUsize>,
}

impl Library {
    pub(crate) fn new(
        ctx: &egui::Context,
        ledger_file: Option<PathBuf>,
        folders: Option<(PathBuf, PathBuf, PathBuf)>,
        parallel: u8,
    ) -> Self {
        let parallel = Arc::new(AtomicUsize::new(usize::from(parallel)));
        let ledger = ledger_file
            .as_deref()
            .and_then(storage::read_text)
            .map(|text| Ledger::from_json(&text))
            .unwrap_or_default();
        let limit = Arc::clone(&parallel);
        let jobs = folders
            .map(|(downloads, apps, data)| Jobs::start(ctx.clone(), downloads, apps, data, limit));
        let mut library = Self {
            ledger,
            ledger_file,
            detected: BTreeMap::new(),
            installed: BTreeMap::new(),
            phases: BTreeMap::new(),
            failures: BTreeMap::new(),
            jobs,
            parallel,
        };
        library.merge();
        library
    }

    pub(crate) fn detect(&mut self, apps: &[App]) {
        self.detected = apps
            .iter()
            .filter(|app| self.ledger.get(&app.id).is_none())
            .filter_map(|app| {
                tgow_platform::detect_installation(&app.id, app.title())
                    .map(|installation| (app.id.clone(), installation))
            })
            .collect();
        self.merge();
    }

    fn merge(&mut self) {
        self.installed = self.detected.clone();
        for installation in &self.ledger.installations {
            self.installed
                .insert(installation.app.clone(), installation.clone());
        }
    }

    fn save(&self) {
        if let Some(file) = &self.ledger_file {
            let _ = storage::write_atomic(file, self.ledger.to_json().as_bytes());
        }
    }

    pub(crate) fn self_update(&mut self, app: &App, platform: &Platform) {
        let Some(manifest) = app.manifest() else {
            return;
        };
        let (Some(tag), Some(jobs)) = (app.tag.clone(), &mut self.jobs) else {
            return;
        };
        let kind = if platform.os == tgow_core::platform::Os::Windows {
            tgow_core::asset::Kind::Exe
        } else {
            tgow_core::asset::Kind::AppImage
        };
        let preference = Preference {
            installed: Some(kind),
            chosen: None,
            system_packages: false,
        };
        let Ok(choice) = select::choose(&manifest.assets, platform, preference) else {
            return;
        };
        if choice.asset.kind != kind {
            return;
        }
        self.phases.insert(app.id.clone(), Phase::Waiting);
        jobs.submit(
            &app.id,
            Task::SelfUpdate(Order {
                app: app.id.clone(),
                name: manifest.name.clone(),
                repository: app.repository.clone(),
                tag,
                version: manifest.version.to_string(),
                asset: choice.asset.clone(),
            }),
        );
    }

    pub(crate) fn poll(&mut self) -> Vec<(String, String)> {
        let Some(jobs) = &mut self.jobs else {
            return Vec::new();
        };
        let reports = jobs.poll();
        let mut finished = Vec::new();
        for report in reports {
            match report {
                Report::Phase(app, phase) => {
                    self.phases.insert(app, phase);
                }
                Report::LauncherUpdated(version) => {
                    self.phases.remove(tgow_core::catalog::LAUNCHER_REPO);
                    finished.push((tgow_core::catalog::LAUNCHER_REPO.to_owned(), version));
                }
                Report::Installed(installation) => {
                    finished.push((installation.app.clone(), installation.version.clone()));
                    self.phases.remove(&installation.app);
                    self.failures.remove(&installation.app);
                    self.detected.remove(&installation.app);
                    self.ledger.record(installation);
                    self.save();
                    self.merge();
                }
                Report::Removed(app) => {
                    self.phases.remove(&app);
                    self.detected.remove(&app);
                    self.ledger.forget(&app);
                    self.save();
                    self.merge();
                }
                Report::Failed(app, message) => {
                    self.phases.remove(&app);
                    self.failures.insert(app, message);
                }
            }
        }
        finished
    }

    pub(crate) fn install(&mut self, app: &App, platform: &Platform, settings: &Settings) {
        let Some(order) = self.order(app, platform, settings) else {
            return;
        };
        let Some(jobs) = &mut self.jobs else {
            return;
        };
        self.failures.remove(&app.id);
        self.phases.insert(app.id.clone(), Phase::Waiting);
        jobs.submit(&app.id, Task::Install(order));
    }

    pub(crate) fn uninstall(&mut self, app: &str) {
        let (Some(installation), Some(jobs)) = (self.installed.get(app).cloned(), &mut self.jobs)
        else {
            return;
        };
        self.phases.insert(app.to_owned(), Phase::Removing);
        jobs.submit(app, Task::Uninstall(installation));
    }

    pub(crate) fn set_parallel(&self, downloads: u8) {
        self.parallel
            .store(usize::from(downloads), Ordering::Relaxed);
    }

    pub(crate) fn is_busy(&self) -> bool {
        !self.phases.is_empty()
    }

    pub(crate) fn cancel(&mut self, app: &str) {
        if let Some(jobs) = &mut self.jobs {
            jobs.cancel(app);
        }
    }

    pub(crate) fn launch(&self, app: &str, settings: &Settings) -> bool {
        let Some(program) = self
            .installed
            .get(app)
            .and_then(|installation| installation.launch.as_ref())
        else {
            return false;
        };
        let arguments = settings
            .apps
            .get(app)
            .map(|preference| preference.arguments.clone())
            .unwrap_or_default();
        process::launch(program, &arguments).is_ok()
    }

    pub(crate) fn reveal(&self, app: &str) {
        if let Some(installation) = self.installed.get(app) {
            let _ = process::reveal(&installation.location);
        }
    }

    pub(crate) fn update_outdated(
        &mut self,
        apps: &[App],
        platform: &Platform,
        settings: &Settings,
    ) {
        let due: Vec<&App> = apps
            .iter()
            .filter(|app| settings.update_mode(&app.id) == UpdateMode::Automatic)
            .filter(|app| {
                !self.phases.contains_key(&app.id) && !self.failures.contains_key(&app.id)
            })
            .filter(|app| {
                let (Some(installation), Some(manifest)) =
                    (self.installed.get(&app.id), app.manifest())
                else {
                    return false;
                };
                installation.is_outdated(manifest)
                    && !installation
                        .launch
                        .as_deref()
                        .is_some_and(process::is_running)
            })
            .collect();
        for app in due {
            self.install(app, platform, settings);
        }
    }

    fn order(&self, app: &App, platform: &Platform, settings: &Settings) -> Option<Order> {
        let manifest = app.manifest()?;
        let tag = app.tag.clone()?;
        let preference = Preference {
            installed: self
                .installed
                .get(&app.id)
                .map(|installation| installation.kind),
            chosen: None,
            system_packages: settings.system_packages,
        };
        let choice = select::choose(&manifest.assets, platform, preference).ok()?;
        Some(Order {
            app: app.id.clone(),
            name: manifest.name.clone(),
            repository: app.repository.clone(),
            tag,
            version: manifest.version.to_string(),
            asset: choice.asset.clone(),
        })
    }
}
