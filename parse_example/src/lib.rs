use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Token{
    Number(i32),
    Operator(char),
    Eof
}

#[derive(Debug)]
pub enum Expression {
    Number(i32),
    Operator(char,Vec<Expression>)
}

#[derive(Debug)]
pub struct Lexer {
    pub tokens: Vec<Token>,
    index : usize
}

impl Lexer {
    pub fn new(input:&str) -> Self {
        Lexer {
            tokens: Lexer::tokenize(input),
            index: 0,
        }
    }

    fn tokenize(input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_whitespace() {
                continue; // Skip whitespace
            }
            match c {
                '0'..='9' => {
                    let mut num = c.to_digit(10).unwrap() as i32;
                    while let Some(next) = chars.peek() {
                        if next.is_numeric() {
                            num = num * 10 + next.to_digit(10).unwrap() as i32;
                            chars.next(); // Consume the digit
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::Number(num));
                }
                _ => {
                    tokens.push(Token::Operator(c));
                }
            }
        }
        tokens.push(Token::Eof);
        tokens
    }

    fn next(&mut self) -> &Token {
        let token = &self.tokens[self.index];
        self.index += 1;
        token
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.index]
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Number(n) => write!(f, "{n}"),
            Expression::Operator(op, exprs) => {
                write!(f, "({op}")?;
                for s in exprs {
                    write!(f, " {}", s)?;
                }
                write!(f, ")")
            }
        }
    }
}


///precedence
/// operator
/// + -  => 1
/// * /  => 2
///
/// Returns a tuple with the precedence and associativity of the operator
/// This function determines the precedence and associativity of the given operator.
/// It returns a tuple where the first element is the precedence level and the second element is the associativity level.
fn precedence(op: char) -> (f32,f32) {
    match op {
        '+' | '-' => (1.0,1.1),
        '*' | '/' => (2.0,2.1),
        '^' => (3.0,3.1),
        _ => panic!("Unknown operator {:?}",op),
    }
}

impl Expression {
    pub fn from_str(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        Self::parse_expression(&mut lexer, 0.0)
    }
    //1 + 2 * 3
    fn parse_expression(lexer: &mut Lexer, min_op : f32) -> Self {
        //println!("current token : {:?}", lexer.peek());

        let mut left_expr = match lexer.next() {
            Token::Number(num) => Expression::Number(*num),
            Token::Operator('(') => {
                let left = Self::parse_expression(lexer, 0.0);
                assert_eq!(lexer.next(), &Token::Operator(')')); 
                left
            },
            t => panic!("left_expr unexpected token: {:?}", t),
        };
        loop {
            let op = match lexer.peek() {
                Token::Eof | Token::Operator(')') => break,
                Token::Operator(op) => *op,
                t => panic!("op unexpected token: {:?}", t),
            };
            let (leaf_pd, right_pd) = precedence(op);
            if leaf_pd < min_op {
                break;
            }
            lexer.next();
            let right_expr = Self::parse_expression(lexer, right_pd);
            left_expr = Expression::Operator(op, vec![left_expr, right_expr]);
        }

        left_expr
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_expression() {
        let lexer = Lexer::new("13 + 5");
        println!("Tokens: {:?}", lexer.tokens);

        assert_eq!(lexer.tokens.len(), 4); // Number(13), Operator(+), Number(5), Eof

        match &lexer.tokens[0] {
            Token::Number(n) => assert_eq!(*n, 13),
            _ => panic!("Expected Number(13)"),
        }

        match &lexer.tokens[1] {
            Token::Operator(op) => assert_eq!(*op, '+'),
            _ => panic!("Expected Operator(+)"),
        }

        match &lexer.tokens[2] {
            Token::Number(n) => assert_eq!(*n, 5),
            _ => panic!("Expected Number(5)"),
        }

        match &lexer.tokens[3] {
            Token::Eof => {},
            _ => panic!("Expected Eof"),
        }
    }

    #[test]
    fn test_1(){
        let s = Expression::from_str("1");
        assert_eq!(s.to_string(), "1");
    }

    #[test]
    fn test_2(){
        let s = Expression::from_str("1 + 2 * 3");
        assert_eq!(s.to_string(), "(+ 1 (* 2 3))")
    }

    #[test]
    fn test_3(){
        let s = Expression::from_str("1 * 2 * 3");
        assert_eq!(s.to_string(), "(* (* 1 2) 3)")
    }

    #[test]
    fn test_4(){
        let s = Expression::from_str("22 + 33 * 2 * 44 + 1 / 4");
        assert_eq!(s.to_string(), "(+ (+ 22 (* (* 33 2) 44)) (/ 1 4))");
    }


    #[test]
    fn test_5(){
        let s = Expression::from_str("2 + 2 * 5 - 3 / 5 + 5 -3");
        assert_eq!(s.to_string(), "(- (+ (- (+ 2 (* 2 5)) (/ 3 5)) 5) 3)");
    }


    #[test]
    fn test_6(){
        let s = Expression::from_str("(2 + 444) * 5 ");
        assert_eq!(s.to_string(), "(* (+ 2 444) 5)");
    }

    #[test]
    fn test_7(){
        let s = Expression::from_str("(((11)))");
        assert_eq!(s.to_string(), "11");
    }

    #[test]
    fn test_8(){
        let s = Expression::from_str("13 + 5 * 211 - 8 / 4");
        assert_eq!(s.to_string(), "(- (+ 13 (* 5 211)) (/ 8 4))");
    }

    #[test]
    fn test_9(){
        let s = Expression::from_str("11 ^ 22 ^ 2");
        assert_eq!(s.to_string(), "(^ (^ 11 22) 2)");
    }

}
