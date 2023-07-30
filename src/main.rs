// #![allow(dead_code)]
// #![allow(unused_variables)]
use std::{
    env::var_os,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use base64::prelude::{Engine, BASE64_URL_SAFE};
use dirs::cache_dir;
use reqwest::{Client, IntoUrl, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, File},
    io,
};

const URL: &str = "https://archlinux.org/mirrors/status/json/";
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
// const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CACHE_TIMEOUT: Duration = Duration::from_secs(300);
const NAME: &str = "Reflector";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MirrorStatus {
    pub cutoff: i64,
    pub last_check: String,
    pub num_checks: i64,
    pub check_frequency: i64,
    pub urls: Vec<Url>,
    pub version: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Url {
    pub url: String,
    pub protocol: String,
    pub last_sync: Option<String>,
    pub completion_pct: f64,
    pub delay: Option<i64>,
    pub duration_avg: Option<f64>,
    pub duration_stddev: Option<f64>,
    pub score: Option<f64>,
    pub active: bool,
    pub country: String,
    pub country_code: String,
    pub isos: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub details: String,
}

pub async fn get_with_timeout<T: IntoUrl>(url: T, timeout: Duration) -> reqwest::Result<Response> {
    Client::builder()
        .timeout(timeout)
        .build()?
        .get(url)
        .send()
        .await
}

async fn get_cache_file(name: &Path) -> io::Result<PathBuf> {
    let mut cache_dir = cache_dir().unwrap_or_else(|| PathBuf::from("/tmp/cache"));
    create_dir_all(cache_dir.as_path()).await?;
    cache_dir.push(name);
    Ok(cache_dir)
}

#[derive(Debug, Error)]
pub enum GetMirrorStatusError {
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("Failed to retrieve")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to serialize/deserialize")]
    Serde(#[from] serde_json::Error),
}

async fn get_mirror_status(
    url: &str,
    cache_timeout: Option<Duration>,
) -> Result<(MirrorStatus, Option<SystemTime>), GetMirrorStatusError> {
    let cache_timeout = cache_timeout.unwrap_or(DEFAULT_CACHE_TIMEOUT);
    let cache_path = (if url == URL {
        get_cache_file(Path::new("mirrorstatus.json")).await
    } else {
        let filename = BASE64_URL_SAFE.encode(url) + ".json";
        get_cache_file(&Path::new(NAME).join(filename)).await
    })?;

    let (mut mtime, invalid) = if cache_path.exists() {
        let mtime = cache_path.metadata()?.created()?;
        (Some(mtime), invalidated(mtime, cache_timeout))
    } else {
        (None, true)
    };
    if invalid {
        let connection_timeout = DEFAULT_CONNECTION_TIMEOUT;
        // reqwest::get(url).await;
        let mirror_status = get_with_timeout(url, connection_timeout)
            .await?
            .json()
            .await?;
        mtime = Some(SystemTime::now());

        serde_json::to_writer(
            File::create(cache_path).await?.into_std().await,
            &mirror_status,
        )?;

        Ok((mirror_status, mtime))
    } else {
        let mirror_status =
            serde_json::from_reader(File::open(cache_path).await?.into_std().await)?;
        Ok((mirror_status, mtime))
    }
}

/// Examing whether `timeout` is passed since `time`.
fn invalidated(time: SystemTime, timeout: Duration) -> bool {
    SystemTime::now().duration_since(time).unwrap_or_default() > timeout
}

#[tokio::main]
async fn main() -> Result<(), GetMirrorStatusError> {
    dbg!(get_mirror_status(URL, None).await?);
    Ok(())
}
