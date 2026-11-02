use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use tgow_core::asset::Asset;
use tgow_core::catalog;
use tgow_core::checksum::Digest;
use tgow_core::install::Installation;
use tgow_net::{Client, Flow};
use tgow_platform::install::{self, Target};
use tgow_ui::shell::Phase;

const PROGRESS_STEP: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub(crate) struct Order {
    pub(crate) app: String,
    pub(crate) name: String,
    pub(crate) repository: String,
    pub(crate) tag: String,
    pub(crate) version: String,
    pub(crate) asset: Asset,
}

pub(crate) enum Task {
    Install(Order),
    Uninstall(Installation),
    SelfUpdate(Order),
}

pub(crate) enum Report {
    Phase(String, Phase),
    LauncherUpdated(String),
    Installed(Installation),
    Removed(String),
    Failed(String, String),
}

pub(crate) struct Jobs {
    tasks: Sender<(Task, Arc<AtomicBool>)>,
    reports: Receiver<Report>,
    cancels: Vec<(String, Arc<AtomicBool>)>,
}

impl Jobs {
    pub(crate) fn start(
        ctx: egui::Context,
        downloads: PathBuf,
        apps: PathBuf,
        data: PathBuf,
        limit: Arc<AtomicUsize>,
    ) -> Self {
        let (tasks, inbox) = mpsc::channel::<(Task, Arc<AtomicBool>)>();
        let (outbox, reports) = mpsc::channel();
        let _ = thread::Builder::new()
            .name("jobs".to_owned())
            .spawn(move || {
                let worker = Arc::new(Worker {
                    client: Client::new(),
                    downloads,
                    apps,
                    data,
                    outbox: Mutex::new(outbox),
                    ctx,
                    gate: Gate {
                        limit,
                        active: Mutex::new(0),
                        changed: Condvar::new(),
                    },
                    installer: Mutex::new(()),
                });
                for (task, cancelled) in inbox {
                    let worker = Arc::clone(&worker);
                    let _ = thread::Builder::new()
                        .name("job".to_owned())
                        .spawn(move || worker.handle(task, &cancelled));
                }
            });
        Self {
            tasks,
            reports,
            cancels: Vec::new(),
        }
    }

    pub(crate) fn submit(&mut self, app: &str, task: Task) {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.cancels.retain(|(known, _)| known != app);
        self.cancels.push((app.to_owned(), Arc::clone(&cancelled)));
        let _ = self.tasks.send((task, cancelled));
    }

    pub(crate) fn cancel(&mut self, app: &str) {
        for (known, flag) in &self.cancels {
            if known == app {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn poll(&mut self) -> Vec<Report> {
        let reports: Vec<Report> = self.reports.try_iter().collect();
        for report in &reports {
            if let Report::Installed(Installation { app, .. })
            | Report::Removed(app)
            | Report::Failed(app, _) = report
            {
                self.cancels.retain(|(known, _)| known != app);
            }
        }
        reports
    }
}

struct Gate {
    limit: Arc<AtomicUsize>,
    active: Mutex<usize>,
    changed: Condvar,
}

struct Pass<'a>(&'a Gate);

impl Gate {
    fn enter(&self) -> Pass<'_> {
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        while *active >= self.limit.load(Ordering::Relaxed).max(1) {
            active = self
                .changed
                .wait(active)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *active += 1;
        Pass(self)
    }
}

impl Drop for Pass<'_> {
    fn drop(&mut self) {
        let mut active = self.0.active.lock().unwrap_or_else(PoisonError::into_inner);
        *active = active.saturating_sub(1);
        self.0.changed.notify_all();
    }
}

struct Worker {
    client: Client,
    downloads: PathBuf,
    apps: PathBuf,
    data: PathBuf,
    outbox: Mutex<Sender<Report>>,
    ctx: egui::Context,
    gate: Gate,
    installer: Mutex<()>,
}

impl Worker {
    fn serial(&self) -> MutexGuard<'_, ()> {
        self.installer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn send(&self, report: Report) {
        let _ = self
            .outbox
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(report);
        self.ctx.request_repaint();
    }

    fn handle(&self, task: Task, cancelled: &AtomicBool) {
        match task {
            Task::Install(order) => {
                let app = order.app.clone();
                match self.install(&order, cancelled) {
                    Ok(installation) => self.send(Report::Installed(installation)),
                    Err(message) => self.send(Report::Failed(app, message)),
                }
            }
            Task::SelfUpdate(order) => {
                let app = order.app.clone();
                match self.self_update(&order, cancelled) {
                    Ok(()) => self.send(Report::LauncherUpdated(order.version.clone())),
                    Err(message) => self.send(Report::Failed(app, message)),
                }
            }
            Task::Uninstall(installation) => {
                let app = installation.app.clone();
                self.send(Report::Phase(app.clone(), Phase::Removing));
                let _serial = self.serial();
                match install::uninstall(&installation) {
                    Ok(()) => self.send(Report::Removed(app)),
                    Err(error) => self.send(Report::Failed(app, error.to_string())),
                }
            }
        }
    }

    fn self_update(&self, order: &Order, cancelled: &AtomicBool) -> Result<(), String> {
        let file = self.fetch(order, cancelled)?;
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let guard = crate::update::Guard::new(&self.data, &executable);
        guard
            .replace(&executable, &file, &order.version)
            .map_err(|error| error.to_string())?;
        let _ = fs::remove_file(&file);
        Ok(())
    }

    fn install(&self, order: &Order, cancelled: &AtomicBool) -> Result<Installation, String> {
        let file = self.fetch(order, cancelled)?;
        let _serial = self.serial();
        self.send(Report::Phase(order.app.clone(), Phase::Installing));
        let target = Target {
            app: &order.app,
            name: &order.name,
            version: &order.version,
            root: &self.apps,
        };
        let installation =
            install::install(&order.asset, &file, &target).map_err(|error| error.to_string())?;
        let _ = fs::remove_file(&file);
        Ok(installation)
    }

    fn fetch(&self, order: &Order, cancelled: &AtomicBool) -> Result<PathBuf, String> {
        let _pass = self.gate.enter();
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".to_owned());
        }
        let folder = self.downloads.join(&order.app);
        fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let file = folder.join(&order.asset.file);
        let partial = folder.join(format!("{}.part", order.asset.file));
        let url = catalog::download_url(&order.repository, &order.tag, &order.asset.file);
        self.send(Report::Phase(
            order.app.clone(),
            Phase::Downloading {
                done: 0,
                total: Some(order.asset.size),
            },
        ));
        let mut last = Instant::now();
        let result = self.client.download(&url, &partial, |done, total| {
            if cancelled.load(Ordering::Relaxed) {
                return Flow::Stop;
            }
            if last.elapsed() >= PROGRESS_STEP {
                last = Instant::now();
                self.send(Report::Phase(
                    order.app.clone(),
                    Phase::Downloading {
                        done,
                        total: total.or(Some(order.asset.size)),
                    },
                ));
            }
            if done > order.asset.size {
                Flow::Stop
            } else {
                Flow::Continue
            }
        });
        if let Err(error) = result {
            if cancelled.load(Ordering::Relaxed) {
                let _ = fs::remove_file(&partial);
            }
            return Err(error.to_string());
        }
        self.send(Report::Phase(order.app.clone(), Phase::Verifying));
        let verified = File::open(&partial)
            .and_then(Digest::of_reader)
            .is_ok_and(|digest| digest == order.asset.sha256)
            && partial
                .metadata()
                .is_ok_and(|metadata| metadata.len() == order.asset.size);
        if !verified {
            let _ = fs::remove_file(&partial);
            return Err("download does not match the signed checksum".to_owned());
        }
        fs::rename(&partial, &file).map_err(|error| error.to_string())?;
        Ok(file)
    }
}
