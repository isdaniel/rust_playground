use std::{env, process};
use grepSample::*;

fn main() {
    let confg = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = run(confg) {
        println!("Application error: {}", e);
        process::exit(1);
    }
    
}


// fn paser_conifg(args: &[String]) -> Config {
//     let filename = &args[1];
//     let query = &args[2];
//     Config { query: query.to_string(), filename: filename.to_string() }
// }
