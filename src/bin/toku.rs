use clap::Parser;

use toku::app;

fn main() -> toku::Result<()> {
    let args = toku::app::Args::parse();
    app::main(args)
}
// use lazy_static::lazy_static;
// use tracing::info;
// use tracing_subscriber;

// lazy_static! {
//     pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_string();
//     pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone().to_uppercase());
//     pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
// }

// fn main() {
//     let xdg_dirs = xdg::BaseDirectories::with_prefix(PROJECT_NAME.clone())
//         .expect("cannot determine XDG paths");
//     let log_path = xdg_dirs
//         .place_data_file(LOG_FILE.clone())
//         .expect("cannot create data file");
//     let log_file = std::fs::File::create(log_path).unwrap();
//     tracing_subscriber::fmt()
//         .with_max_level(tracing::Level::TRACE)
//         .with_writer(log_file)
//         .init();

//     let number_of_yaks = 3;
//     // this creates a new event, outside of any spans.
//     info!(number_of_yaks, "preparing to shave yaks");

//     let number_shaved = 3;
//     info!(all_yaks_shaved = number_shaved == number_of_yaks, "yak shaving completed.");
// }
