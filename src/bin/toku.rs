use clap::Parser;

use toku::app;

fn main() -> app::Result<()> {
    let args = toku::app::Args::parse();
    app::main(args)
}
