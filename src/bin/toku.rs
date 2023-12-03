use anyhow::Result;
use clap::Parser;
use toku::app;

fn main() -> Result<()> {
    let args = toku::app::Args::parse();
    app::main(args)
}
