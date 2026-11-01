use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
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
}

pub(crate) enum Report {
    Phase(String, Phase),
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
    pub(crate) fn start(ctx: egui::Context, downloads: PathBuf, apps: PathBuf) -> Self {
        let (tasks, inbox) = mpsc::channel::<(Task, Arc<AtomicBool>)>();
        let (outbox, reports) = mpsc::channel();
        let _ = thread::Builder::new()
            .name("jobs".to_owned())
            .spawn(move || {
                let worker = Worker {
                    client: Client::new(),
                    downloads,
                    apps,
                    outbox,
                    ctx,
                };
                for (task, cancelled) in inbox {
                    worker.handle(task, &cancelled);
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

struct Worker {
    client: Client,
    downloads: PathBuf,
    apps: PathBuf,
    outbox: Sender<Report>,
    ctx: egui::Context,
}

impl Worker {
    fn send(&self, report: Report) {
        let _ = self.outbox.send(report);
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
            Task::Uninstall(installation) => {
                let app = installation.app.clone();
                self.send(Report::Phase(app.clone(), Phase::Removing));
                match install::uninstall(&installation) {
                    Ok(()) => self.send(Report::Removed(app)),
                    Err(error) => self.send(Report::Failed(app, error.to_string())),
                }
            }
        }
    }

    fn install(&self, order: &Order, cancelled: &AtomicBool) -> Result<Installation, String> {
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
}
