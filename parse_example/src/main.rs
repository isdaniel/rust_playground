use std::{collections::HashMap, io::{self, Write}};
use parse_example::*;

fn main() {
    let mut variables: HashMap<char, i32> = HashMap::new();
    loop{
        print!(">> ");
        io::stdout().flush().unwrap();
        let input = {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).unwrap();
            buf
        };
        if input.trim() == "exit" {
            break;
        }
        let expr = Expression::from_str(&input);
        if let Some((var_name, lhs)) = expr.is_asign(){
            let value = lhs.eval(&variables);
            variables.insert(var_name, value);
            continue;
        }
        let value = expr.eval(&variables);
        println!("{}", value);
    }
}

