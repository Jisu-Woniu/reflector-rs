#[path = "src/cli.rs"]
mod cli;
use clap::CommandFactory;
use clap_complete::{
    generate_to,
    shells::{Bash, Fish, Zsh},
};
use clap_mangen::Man;
use cli::Arguments;
use lazy_static::lazy_static;
use std::{
    env::var_os,
    fs::File,
    io::{BufWriter, Result},
    path::PathBuf,
};
use url::Url;

lazy_static! {
    static ref DEFAULT_URL: Url = Url::parse("https://archlinux.org/mirrors/status/json/").unwrap();
}

fn main() -> Result<()> {
    let outdir = PathBuf::from(var_os("OUT_DIR").unwrap());
    generate_to(Bash, &mut Arguments::command(), "reflector-rs", &outdir)?;
    generate_to(Zsh, &mut Arguments::command(), "reflector-rs", &outdir)?;
    generate_to(Fish, &mut Arguments::command(), "reflector-rs", &outdir)?;
    let man = Man::new(Arguments::command());
    man.render(&mut BufWriter::new(File::create(
        outdir.join("reflector-rs.1"),
    )?))?;
    Ok(())
}
