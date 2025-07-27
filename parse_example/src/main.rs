use parse_example::*;

fn main() {
    let input = "(13 + 5) * 211 - 8 / 4";
    let lexer = Lexer::new(input);
    println!("lexer: {:?}",lexer);
    let expression = Expression::from_str(input);
    println!("expression: {:?}",expression.to_string());
    println!("{input} = {}",expression.eval());
}
