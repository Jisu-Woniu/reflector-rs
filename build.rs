use std::{
    env::var_os,
    fs::File,
    io::{BufWriter, Result},
    path::PathBuf,
};

use clap::CommandFactory;
use clap_complete::{
    generate_to,
    shells::{Bash, Fish, Zsh},
};
use clap_mangen::Man;

use crate::cli::Arguments;

#[path = "src/cli.rs"]
mod cli;

fn main() -> Result<()> {
    let outdir = PathBuf::from(var_os("OUT_DIR").unwrap());
    const BIN_NAME: &str = "reflector-rs";
    generate_to(Bash, &mut Arguments::command(), BIN_NAME, &outdir)?;
    generate_to(Zsh, &mut Arguments::command(), BIN_NAME, &outdir)?;
    generate_to(Fish, &mut Arguments::command(), BIN_NAME, &outdir)?;

    let man = Man::new(Arguments::command());
    man.render(&mut BufWriter::new(File::create(
        outdir.join("reflector-rs.1"),
    )?))?;

    Ok(())
}
