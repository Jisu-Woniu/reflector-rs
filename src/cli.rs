use std::{path::PathBuf, sync::LazyLock};

use clap::{Parser, ValueEnum};
use url::Url;

pub static DEFAULT_URL: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://archlinux.org/mirrors/status/json/").unwrap());

const DEFAULT_CONNECTION_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_DOWNLOAD_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_CACHE_TIMEOUT_SECONDS: u64 = 300;

/// retrieve and filter a list of the latest Arch Linux mirrors
#[derive(Debug, Parser)]
#[command()]
pub(crate) struct Arguments {
    /// The number of seconds to wait before a connection times out.
    #[arg(long, default_value_t = DEFAULT_CONNECTION_TIMEOUT_SECONDS)]
    pub(crate) connection_timeout: u64,

    /// The number of seconds to wait before a download times out.
    #[arg(long, default_value_t = DEFAULT_DOWNLOAD_TIMEOUT_SECONDS)]
    pub(crate) download_timeout: u64,

    /// Display a table of the distribution of servers by country.
    #[arg(long)]
    pub(crate) list_countries: bool,

    /// The cache timeout in seconds for the data retrieved from the Arch Linux Mirror Status API.
    #[arg(long, default_value_t = DEFAULT_CACHE_TIMEOUT_SECONDS)]
    pub(crate) cache_timeout: u64,

    /// The URL from which to retrieve the mirror data in JSON format.
    ///
    /// If different from the default, it must follow the same format.
    #[arg(long, default_value_t = DEFAULT_URL.clone())]
    pub(crate) url: Url,

    /// Save the mirrorlist to the given path.
    #[arg(long)]
    pub(crate) save: Option<PathBuf>,

    /// Sort the mirrorlist.
    #[arg(long)]
    pub(crate) sort: Option<SortType>,

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
    pub(crate) threads: Option<u64>,

    #[command(flatten, next_help_heading = "Filters")]
    pub(crate) filters: Filters,
}

/// The following filters are inclusive, i.e. the returned list will only
/// contain mirrors for which all of the given conditions are met.
#[derive(Clone, Debug, Parser)]
#[group(multiple = true)]
pub(crate) struct Filters {
    /// Only return mirrors that have synchronized in the last n hours.
    ///
    /// n may be a float.
    #[arg(short, long, value_name = "n")]
    pub(crate) age: Option<f64>,

    /// Only return mirrors with a reported sync delay of n hours or
    /// less, where n is a float.
    ///
    /// For example, to limit the results to
    /// mirrors with a reported delay of 15 minutes or less, pass 0.25.
    #[arg(long, value_name = "n")]
    pub(crate) delay: Option<f64>,

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
    pub(crate) country: Option<Vec<String>>,

    /// Return the n fastest mirrors that meet the other criteria.
    /// Do not use this option without other filtering options.
    #[arg(short, long, value_name = "n")]
    pub(crate) fastest: Option<usize>,

    /// Include servers that match <regex>, where <regex> is a regular expression.
    #[arg(short, long, value_name = "regex")]
    pub(crate) include: Option<String>,

    /// Exclude servers that match <regex>, where <regex> is a regular expression.
    #[arg(short = 'x', long, value_name = "regex")]
    pub(crate) exclude: Option<String>,

    /// Limit the list to the n most recently synchronized servers.
    #[arg(short, long, value_name = "n")]
    pub(crate) latest: Option<usize>,

    /// Limit the list to the n servers with the highest score.
    #[arg(long, value_name = "n")]
    pub(crate) score: Option<usize>,

    /// Return at most n mirrors.
    #[arg(short, long, value_name = "n")]
    pub(crate) number: Option<usize>,

    /// Match one of the given protocols.
    ///
    /// Multiple protocols may be selected using commas (e.g. "https,http")
    /// or by passing this option multiple times.
    #[arg(short, long)]
    pub(crate) protocols: Option<Vec<Protocol>>,

    /// Set the minimum completion percent for the returned mirrors.
    ///
    /// Check the mirrorstatus webpage for the meaning of this parameter.
    #[arg(long, default_value_t = 100.0)]
    pub(crate) completion_percent: f64,

    /// Only return mirrors that host ISOs.
    #[arg(long)]
    pub(crate) isos: bool,

    /// Only return mirrors that support IPv4.
    #[arg(long)]
    pub(crate) ipv4: bool,

    /// Only return mirrors that support IPv6.
    #[arg(long)]
    pub(crate) ipv6: bool,
}

#[derive(Clone, Debug, ValueEnum)]
#[value()]
pub(crate) enum SortType {
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
pub(crate) enum Protocol {
    Ftp,
    Http,
    Https,
    Rsync,
}
mod tests {

    #[test]
    fn verify_args() {
        use clap::CommandFactory;

        use super::Arguments;
        Arguments::command().debug_assert()
    }
}
