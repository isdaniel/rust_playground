use std::{fs, error::Error};

pub struct Config {
    pub query: String,
    pub filename: String,
}

pub fn run(config : Config) -> Result<(),Box<dyn Error>> {
    let contents = fs::read_to_string(config.filename)?;
    println!("With text:\n{}", contents);
    Ok(())
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }
        let filename = &args[1];
        let query = &args[2];
        Ok( Config { query: query.to_string(), filename: filename.to_string() })
    }
}
