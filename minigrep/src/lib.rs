use std::fs;
use std::error::Error;

pub struct Config {
    pub query: String,
    pub file_path: String,
    pub ignore_case: bool
}

impl Config{
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {

        args.next();

        let query = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a query string")
        };

        let file_path = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a query string")
        };

        let ignore_case = std::env::var("IGNORE_CASE").is_ok();
        Ok(Config { query, file_path,ignore_case })
    }
}

pub fn search<'a>(query: &str, content: &'a str) -> Vec<&'a str> {
    content.lines()
        .filter(|line| line.contains(query))
        .collect()
}

pub fn search_case_insensitive<'a>(query: &str, content: &'a str) -> Vec<&'a str> {
    content.lines()
        .filter(|line| line.to_lowercase().contains(&query.to_lowercase()))
        .collect()
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(config.file_path)?;
    let searchword = if config.ignore_case{
        search_case_insensitive(&config.query, &content)
    } else{
        search(&config.query, &content)
    };

    for line in searchword {
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn case_sensitive(){
        let query = "test";
        let content = "\
Rust
test
Hello world!";

        assert_eq!(vec![query], search(query, content));
    }

    #[test]
    fn case_insensitive(){
        let query = "TEST";
        let content = "\
Rust
test
Test
Hello world!";

        assert_eq!(vec!["test","Test"], search_case_insensitive(query, content));
    }
}