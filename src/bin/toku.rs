use clap::Parser;

use toku::app;

fn main() -> toku::Result<()> {
    let args = toku::app::Args::parse();
    app::main(args)
}
