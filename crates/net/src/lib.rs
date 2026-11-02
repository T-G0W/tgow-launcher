#![forbid(unsafe_code)]

mod host;
mod limit;

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tgow_core::catalog::{self, OWNER};
use ureq::Agent;
use ureq::http::{Response, StatusCode};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

pub use crate::host::{ALLOWED_HOSTS, is_allowed};
pub use crate::limit::retry_after;

const USER_AGENT: &str = "tgow-launcher";
const API_VERSION: &str = "2022-11-28";
const MAX_REDIRECTS: usize = 5;
const REPOSITORY_LIST_LIMIT: u64 = 2 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

#[derive(Debug)]
pub enum Error {
    Offline(String),
    Status(u16),
    RateLimited(Duration),
    ForbiddenHost(String),
    TooLarge,
    Io(io::Error),
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline(reason) => write!(f, "github is not reachable: {reason}"),
            Self::Status(status) => write!(f, "github answered with status {status}"),
            Self::RateLimited(wait) => {
                write!(f, "github rate limit, retry in {} s", wait.as_secs())
            }
            Self::ForbiddenHost(host) => write!(f, "refused to contact {host}"),
            Self::TooLarge => f.write_str("response is larger than allowed"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Cancelled => f.write_str("cancelled"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ureq::Error> for Error {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::BodyExceedsLimit(_) => Self::TooLarge,
            ureq::Error::Io(error) => Self::Io(error),
            other => Self::Offline(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetch {
    Fresh { body: Vec<u8>, etag: Option<String> },
    NotModified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

#[derive(Clone)]
pub struct Client {
    agent: Agent,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    #[must_use]
    pub fn new() -> Self {
        let tls = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .root_certs(RootCerts::PlatformVerifier)
            .build();
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .https_only(true)
            .max_redirects(0)
            .user_agent(USER_AGENT)
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_recv_response(Some(Duration::from_secs(15)))
            .timeout_per_read(Some(Duration::from_secs(30)))
            .timeout_per_write(Some(Duration::from_secs(30)))
            .tls_config(tls)
            .build()
            .into();
        Self { agent }
    }

    pub fn repositories(&self, etag: Option<&str>) -> Result<Fetch, Error> {
        let url = format!(
            "https://api.github.com/users/{OWNER}/repos?per_page=100&type=owner&sort=pushed"
        );
        guard(&url)?;
        let mut request = self
            .agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION);
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request.call()?;
        match response.status() {
            StatusCode::NOT_MODIFIED => Ok(Fetch::NotModified),
            StatusCode::OK => {
                let etag = header(&response, "etag");
                let body = response
                    .body_mut()
                    .with_config()
                    .limit(REPOSITORY_LIST_LIMIT)
                    .read_to_vec()?;
                Ok(Fetch::Fresh { body, etag })
            }
            status => Err(failure(&response, status)),
        }
    }

    pub fn latest_tag(&self, repository: &str) -> Result<Option<String>, Error> {
        let url = catalog::latest_url(repository);
        guard(&url)?;
        let response = self.agent.head(&url).call()?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            status if status.is_redirection() => Ok(header(&response, "location")
                .and_then(|location| catalog::tag_from_location(repository, &location))),
            status => Err(failure(&response, status)),
        }
    }

    pub fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, Error> {
        let mut response = self.follow(url, 0)?;
        Ok(response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()?)
    }

    pub fn download(
        &self,
        url: &str,
        file: &Path,
        mut progress: impl FnMut(u64, Option<u64>) -> Flow,
    ) -> Result<u64, Error> {
        let offset = file.metadata().map_or(0, |metadata| metadata.len());
        let mut response = self.follow(url, offset)?;
        let resumed = response.status() == StatusCode::PARTIAL_CONTENT;
        let start = if resumed { offset } else { 0 };
        let total = response
            .body()
            .content_length()
            .map(|length| length + start);
        let mut output = if resumed {
            OpenOptions::new().append(true).open(file)?
        } else {
            File::create(file)?
        };
        let mut reader = response.body_mut().as_reader();
        let mut buffer = vec![0; CHUNK];
        let mut written = start;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
            written += u64::try_from(read).unwrap_or(0);
            if progress(written, total) == Flow::Stop {
                output.flush()?;
                return Err(Error::Cancelled);
            }
        }
        output.flush()?;
        Ok(written)
    }

    fn follow(&self, url: &str, offset: u64) -> Result<Response<ureq::Body>, Error> {
        let mut current = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            guard(&current)?;
            let mut request = self.agent.get(&current);
            if offset > 0 {
                request = request.header("Range", format!("bytes={offset}-"));
            }
            let response = request.call()?;
            let status = response.status();
            if status.is_redirection() {
                current = header(&response, "location").ok_or(Error::Status(status.as_u16()))?;
                continue;
            }
            if status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT {
                return Ok(response);
            }
            return Err(failure(&response, status));
        }
        Err(Error::Status(310))
    }
}

fn guard(url: &str) -> Result<(), Error> {
    if is_allowed(url) {
        Ok(())
    } else {
        Err(Error::ForbiddenHost(
            host::name(url).unwrap_or("unknown").to_owned(),
        ))
    }
}

fn header<T>(response: &Response<T>, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn failure<T>(response: &Response<T>, status: StatusCode) -> Error {
    if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let wait = retry_after(
            header(response, "retry-after").as_deref(),
            header(response, "x-ratelimit-remaining").as_deref(),
            header(response, "x-ratelimit-reset").as_deref(),
            now,
        );
        if let Some(wait) = wait {
            return Error::RateLimited(wait);
        }
    }
    Error::Status(status.as_u16())
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    #[ignore = "talks to github.com"]
    fn reads_the_latest_ferrovox_release_without_api_quota() {
        let client = Client::new();
        let tag = client.latest_tag("ferrovox").unwrap().unwrap();
        assert!(tag.starts_with('v'));
        let url = tgow_core::catalog::download_url("ferrovox", &tag, "latest.json");
        let manifest = client.fetch(&url, 256 * 1024).unwrap();
        assert!(manifest.starts_with(b"{"));
    }
}
