use std::env;
use minigrep::run;
use minigrep::Config;

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        std::process::exit(1);
    });

    if let Err(e) = run(config){
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }
}


