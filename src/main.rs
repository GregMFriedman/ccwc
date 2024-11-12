use std::env;
use std::process;

use gfwc::{Config, Counter};

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    let counter = Counter::from(config);
    if let Err(e) = counter.count() {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
