// #![allow(dead_code)]
// #![allow(unused_variables)]
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use base64::prelude::{Engine, BASE64_URL_SAFE};
use clap::{builder::PossibleValue, Parser, ValueEnum};
use dirs::cache_dir;
use reqwest::{Client, IntoUrl, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, File},
    io,
};
const DEFAULT_CONNECTION_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_DOWNLOAD_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_CACHE_TIMEOUT_SECONDS: u64 = 300;
const URL: &str = "https://archlinux.org/mirrors/status/json/";

/// retrieve and filter a list of the latest Arch Linux mirrors
#[derive(Debug, Parser)]
#[command()]
struct Arguments {
    /// The number of seconds to wait before a connection times out.
    #[arg(long, default_value_t = DEFAULT_CONNECTION_TIMEOUT_SECONDS)]
    connection_timeout: u64,

    /// The number of seconds to wait before a download times out.
    #[arg(long, default_value_t = DEFAULT_DOWNLOAD_TIMEOUT_SECONDS)]
    download_timeout: u64,

    /// Display a table of the distribution of servers by country.
    #[arg(long)]
    list_countries: bool,

    /// The cache timeout in seconds for the data retrieved from the Arch Linux Mirror Status API.
    #[arg(long, default_value_t = DEFAULT_CACHE_TIMEOUT_SECONDS)]
    cache_timeout: u64,

    /// The URL from which to retrieve the mirror data in JSON format.
    ///
    /// If different from the default, it must follow the same format.
    #[arg(long, default_value_t = URL.to_string())]
    url: String,

    /// Save the mirrorlist to the given path.
    #[arg(long)]
    save: Option<PathBuf>,

    /// Sort the mirrorlist.
    #[arg(long)]
    sort: Option<SortType>,

    /// Number of threads used for rating mirrors.
    ///
    /// This option will speed up the
    /// rating step but the results will be inaccurate if the local
    /// bandwidth is saturated at any point during the operation. If rating
    /// takes too long without this option then you should probably apply
    /// more filters to reduce the number of rated servers before using this
    /// option.
    #[arg(long)]
    // Refer to Rust thread pool
    threads: Option<u64>,

    #[command(flatten, next_help_heading = "Filters")]
    filters: Filters,
}
/// The following filters are inclusive, i.e. the returned list will only
/// contain mirrors for which all of the given conditions are met.
#[derive(Clone, Debug, Parser)]
#[group(multiple = true)]
struct Filters {
    /// Only return mirrors that have synchronized in the last n hours.
    ///
    /// n may be a float.
    #[arg(short, long, value_name = "n")]
    age: Option<f64>,

    /// Only return mirrors with a reported sync delay of n hours or
    /// less, where n is a float.
    ///
    /// For example, to limit the results to
    /// mirrors with a reported delay of 15 minutes or less, pass 0.25.
    #[arg(long, value_name = "n")]
    delay: Option<f64>,

    /// Restrict mirrors to selected countries.
    ///
    /// Countries may be given by name or country code, or a mix of both.
    /// The case is ignored.
    /// Multiple countries may be selected using commas (e.g. --country
    /// France,Germany) or by passing this option multiple times (e.g.  -c
    /// fr -c de). Use "--list-countries" to display a table of available
    /// countries along with their country codes. When sorting by country,
    /// this option may also be used to sort by a preferred order instead of
    /// alphabetically. For example, to select mirrors from Sweden, Norway,
    /// Denmark and Finland, in that order, use the options "--country
    /// se,no,dk,fi --sort country". To set a preferred country sort order
    /// without filtering any countries.  this option also recognizes the
    /// glob pattern "*", which will match any country. For example, to
    /// ensure that any mirrors from Sweden are at the top of the list and
    /// any mirrors from Denmark are at the bottom, with any other countries
    /// in between, use "--country 'se,*,dk' --sort country". It is
    /// however important to note that when "*" is given along with other
    /// filter criteria, there is no guarantee that certain countries will
    /// be included in the results. For example, with the options "--country
    /// 'se,*,dk' --sort country --latest 10", the latest 10 mirrors may
    /// all be from the United States. When the glob pattern is present, it
    /// only ensures that if certain countries are included in the results,
    /// they will be sorted in the requested order.
    #[arg(short, long)]
    country: Option<Vec<String>>,

    /// Return the n fastest mirrors that meet the other criteria.
    /// Do not use this option without other filtering options.
    #[arg(short, long, value_name = "n")]
    fastest: Option<usize>,

    /// Include servers that match <regex>, where <regex> is a regular expression.
    #[arg(short, long, value_name = "regex")]
    include: Option<String>,

    /// Exclude servers that match <regex>, where <regex> is a regular expression.
    #[arg(short = 'x', long, value_name = "regex")]
    exclude: Option<String>,

    /// Limit the list to the n most recently synchronized servers.
    #[arg(short, long, value_name = "n")]
    latest: Option<usize>,

    /// Limit the list to the n servers with the highest score.
    #[arg(long, value_name = "n")]
    score: Option<usize>,

    /// Return at most n mirrors.
    #[arg(short, long, value_name = "n")]
    number: Option<usize>,

    /// Match one of the given protocols.
    ///
    /// Multiple protocols may be selected using commas (e.g. "https,http")
    /// or by passing this option multiple times.
    #[arg(short, long)]
    protocols: Option<Vec<Protocol>>,

    /// Set the minimum completion percent for the returned mirrors.
    ///
    /// Check the mirrorstatus webpage for the meaning of this parameter.
    #[arg(long, default_value_t = 100.0)]
    completion_percent: f64,

    /// Only return mirrors that host ISOs.
    #[arg(long)]
    isos: bool,

    /// Only return mirrors that support IPv4.
    #[arg(long)]
    ipv4: bool,

    /// Only return mirrors that support IPv6.
    #[arg(long)]
    ipv6: bool,
}

#[derive(Clone, Debug, ValueEnum)]
#[value()]
enum SortType {
    /// last server synchronization
    Age,
    /// download rate
    Rate,
    /// country name, either alphabetically or in the order given by the --country option
    Country,
    /// MirrorStatus score
    Score,
    /// MirrorStatus delay
    Delay,
}

#[derive(Clone, Debug, ValueEnum)]
#[value()]
enum Protocol {
    Ftp,
    Http,
    Https,
    Rsync,
}

const DEFAULT_CONNECTION_TIMEOUT: Duration =
    Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT_SECONDS);
const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(DEFAULT_DOWNLOAD_TIMEOUT_SECONDS);
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
    #[error("I/O error")]
    IO(#[from] io::Error),
    #[error("Failed to retrieve")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to serialize/deserialize")]
    Serde(#[from] serde_json::Error),
}

async fn get_mirror_status(
    connection_timeout: Option<Duration>,
    cache_timeout: Option<Duration>,
    url: &str,
) -> Result<(MirrorStatus, Option<SystemTime>), GetMirrorStatusError> {
    let cache_timeout = cache_timeout.unwrap_or(DEFAULT_CACHE_TIMEOUT);
    let connection_timeout = connection_timeout.unwrap_or(DEFAULT_CONNECTION_TIMEOUT);
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

fn get_logger() {
    unimplemented!()
}

fn split_list_args(args: Vec<String>) -> Vec<String> {
    args.into_iter().flat_map(|arg| arg.split(',')).collect()
}

#[tokio::main]
async fn main() -> Result<(), GetMirrorStatusError> {
    let arguments = Arguments::parse();
    println!("{:#?}", arguments);
    // dbg!(get_mirror_status(None, None, URL).await?.0.urls);
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::future::Future;

    use clap::{Command, CommandFactory};

    use super::*;

    #[test]
    fn verify_args() {
        Arguments::command().debug_assert()
    }

    fn block<F>(future: F) -> F::Output
    where
        F: Future,
    {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn test_get_mirror_status() -> Result<(), GetMirrorStatusError> {
        block(async {
            dbg!(get_mirror_status(None, None, URL).await?.0.urls);
            Ok(())
        })
    }
}
