use std::{fs, error::Error,env};

pub struct Config {
    pub query: String,
    pub filename: String,
    pub case_sensitive: bool,
}

pub fn run(config : Config) -> Result<(),Box<dyn Error>> {
    let contents = fs::read_to_string(config.filename)?;
    for line in search(&config.query, &contents) {
        println!("{}", line);
    }
    
    Ok(())
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }
        args.next();
        let filename = match args.next(){
            Some(arg) => arg,
            None => return Err("Didn't get a file name"),
        };
        let query =match args.next(){
            Some(arg) => arg,
            None => return Err("Didn't get a query string"),
        };

        let case_sensitive = env::var("CASE_INSENSITIVE").is_err();
        Ok( Config { query: query, filename: filename, case_sensitive })
    }
}

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    //let mut result = Vec::new();

    // for line in contents.lines() {
    //     if line.contains(query) {
    //         result.push(line);
    //     }
    // }

    //result

    contents.lines().filter(|line| line.contains(query)).collect()    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_search_contain_hello() {
        let query = "hello";
        let contents = "\
hello world!
hello rust!!
test 123";
        assert_eq!(vec!["hello world!","hello rust!!"], search(query, contents));
    }

    
    #[test]
    fn it_search_contain_test() {
        let query = "test";
        let contents = "\
hello world!
hello rust!!
test 123";
        assert_eq!(vec!["test 123"], search(query, contents));
    }

    #[test]
    fn it_search_contain_empty() {
        let query = "ssss";
        let contents = "\
hello world!
hello rust!!
test 123";
        assert_eq!(Vec::<&str>::new(), search(query, contents));
    }
}