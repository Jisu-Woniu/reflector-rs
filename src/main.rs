#![allow(dead_code)]
// #![allow(unused_variables)]
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use base64::prelude::{Engine, BASE64_URL_SAFE};
use clap::Parser;
use dirs::cache_dir;
use reqwest::{Client, IntoUrl, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, File},
    io,
};
use url::Url;

use lazy_static::lazy_static;
lazy_static! {
    static ref DEFAULT_URL: Url = Url::parse("https://archlinux.org/mirrors/status/json/").unwrap();
}
mod cli;

// const DEFAULT_CONNECTION_TIMEOUT: Duration =
//     Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT_SECONDS);
// const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(DEFAULT_DOWNLOAD_TIMEOUT_SECONDS);
// const DEFAULT_CACHE_TIMEOUT: Duration = Duration::from_secs(300);
const NAME: &str = "Reflector-rs";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MirrorStatus {
    pub cutoff: i64,
    pub last_check: String,
    pub num_checks: i64,
    pub check_frequency: i64,
    pub urls: Vec<Mirror>,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Mirror {
    pub url: Url,
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

/// Send GET request to `url` with reqwest.
async fn get_with_timeout<T: IntoUrl>(url: T, timeout: Duration) -> reqwest::Result<Response> {
    Client::builder()
        .timeout(timeout)
        .build()?
        .get(url)
        .send()
        .await
}

/// Examing whether `timeout` is passed since `time`.
fn invalidated(time: SystemTime, timeout: Duration) -> bool {
    SystemTime::now().duration_since(time).unwrap_or_default() > timeout
}

async fn get_cache_file(name: &Path) -> io::Result<PathBuf> {
    let mut cache_dir = cache_dir().unwrap_or_else(|| PathBuf::from("/tmp/cache"));
    create_dir_all(cache_dir.as_path()).await?;
    cache_dir.push(name);
    Ok(cache_dir)
}

#[derive(Debug, Error)]
pub enum GetMirrorStatusError {
    #[error("I/O error: {0}")]
    IO(#[from] io::Error),
    #[error("Failed to retrieve: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to serialize/deserialize: {0}")]
    Serde(#[from] serde_json::Error),
}

async fn get_mirror_status(
    connection_timeout: Duration,
    cache_timeout: Duration,
    url: &Url,
) -> Result<(MirrorStatus, Option<SystemTime>), GetMirrorStatusError> {
    let cache_timeout = cache_timeout;
    let connection_timeout = connection_timeout;
    let cache_path = (if url == DEFAULT_URL.deref() {
        get_cache_file(Path::new("mirrorstatus.json")).await
    } else {
        let filename = BASE64_URL_SAFE.encode(url.as_str()) + ".json";
        get_cache_file(&Path::new(NAME).join(filename)).await
    })?;

    let (mut mtime, invalid) = if cache_path.exists() {
        let mtime = cache_path.metadata()?.created()?;
        (Some(mtime), invalidated(mtime, cache_timeout))
    } else {
        (None, true)
    };
    if invalid {
        let mirror_status = get_with_timeout(url.clone(), connection_timeout)
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

fn get_logger() {
    unimplemented!()
}

fn split_list_args(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .flat_map(|arg| arg.split(',').map(ToString::to_string).collect::<Vec<_>>())
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), GetMirrorStatusError> {
    let arguments = cli::Arguments::parse();
    println!("{:#?}", arguments);
    // dbg!(get_mirror_status(None, None, URL).await?.0.urls);
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_get_mirror_status() -> Result<(), GetMirrorStatusError> {
        async {
            let mirror_status = get_mirror_status(
                Duration::from_secs(5),
                Duration::from_secs(5),
                DEFAULT_URL.deref(),
            )
            .await?
            .0
            .urls;
            dbg!(mirror_status);
            Ok(())
        }
        .await
    }
}
