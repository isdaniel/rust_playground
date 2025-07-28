use std::{collections::HashMap, fmt};

#[derive(Debug, PartialEq)]
pub enum Token{
    Number(i32),
    Var(char),
    Operator(char),
    Eof
}

#[derive(Debug)]
pub enum Expression {
    Number(i32),
    Var(char),
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
                'a'..='z' | 'A'..='Z' => {
                    tokens.push(Token::Var(c));
                },
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
        if self.index >= self.tokens.len() {
            panic!("No more tokens available");
        }

        let token = &self.tokens[self.index];
        self.index += 1;
        token
    }

    fn peek(&self) -> &Token {
        if self.index >= self.tokens.len() {
            panic!("No more tokens available");
        }
        &self.tokens[self.index]
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Number(n) => write!(f, "{n}"),
            Expression::Var(name) => write!(f, "{name}"),
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
        '=' => (0.0, 0.1),
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
        let mut left_expr = match lexer.next() {
            Token::Number(num) => Expression::Number(*num),
            Token::Var(name) => Expression::Var(*name),
            Token::Operator('(') => {
                let left = Self::parse_expression(lexer, 0.0);
                assert_eq!(lexer.next(), &Token::Operator(')'));
                left
            },
            Token::Operator('-') => {
                let left = Self::parse_expression(lexer, 0.0);
                Expression::Operator('-', vec![Expression::Number(0), left])
            }
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

    pub fn eval(&self, variables: &HashMap<char, i32>) -> i32 {
        match self {
            Expression::Number(n) => *n,
            Expression::Var(name) => {
                *variables.get(name).expect("Variable not found")
            },
            Expression::Operator(op, exprs) => {
                let left_result = exprs[0].eval(variables);
                let right_result = exprs[1].eval(variables);
                let result = match op {
                    '+' => left_result + right_result,
                    '-' => left_result - right_result,
                    '*' => left_result * right_result,
                    '/' => left_result / right_result,
                    '^' => {
                        if right_result < 0 {
                            panic!("Negative exponent not supported");
                        }
                        left_result.pow(right_result as u32)
                    },
                    _ => panic!("Unknown operator: {}", op),
                };
                result
            }
        }
    }

    /// Convenience method to evaluate expressions without variables
    pub fn eval_no_vars(&self) -> i32 {
        let empty_vars = HashMap::new();
        self.eval(&empty_vars)
    }

    /// a = 1
    /// b = a + 1
    pub fn is_asign(&self) -> Option<(char, &Expression)> {
        match self {
            Expression::Operator('=', exprs) if exprs.len() == 2 => {
                if let Expression::Var(var_name) = &exprs[0] {
                    Some((*var_name, &exprs[1]))
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // ===== TOKENIZATION TESTS =====

    #[test]
    fn tokenize_simple_addition_expression() {
        let lexer = Lexer::new("13 + 5");

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
    fn tokenize_multi_digit_numbers() {
        let lexer = Lexer::new("123 + 4567");
        assert_eq!(lexer.tokens[0], Token::Number(123));
        assert_eq!(lexer.tokens[2], Token::Number(4567));
    }

    #[test]
    fn tokenize_all_operators() {
        let lexer = Lexer::new("1 + 2 - 3 * 4 / 5 ^ 6");
        let expected_ops = ['+', '-', '*', '/', '^'];
        let mut op_index = 0;

        for (i, token) in lexer.tokens.iter().enumerate() {
            if let Token::Operator(op) = token {
                if i > 0 { // Skip first token which is a number
                    assert_eq!(*op, expected_ops[op_index]);
                    op_index += 1;
                }
            }
        }
    }

    #[test]
    fn tokenize_expression_with_parentheses() {
        let lexer = Lexer::new("(1 + 2) * 3");
        assert_eq!(lexer.tokens[0], Token::Operator('('));
        assert_eq!(lexer.tokens[4], Token::Operator(')'));
    }

    #[test]
    fn tokenize_expression_with_excessive_whitespace() {
        let lexer = Lexer::new("  1   +   2   ");
        assert_eq!(lexer.tokens.len(), 4); // Should ignore whitespace
        assert_eq!(lexer.tokens[0], Token::Number(1));
        assert_eq!(lexer.tokens[1], Token::Operator('+'));
        assert_eq!(lexer.tokens[2], Token::Number(2));
    }

    // ===== PARSING TESTS =====

    #[test]
    fn parse_single_number() {
        let expr = Expression::from_str("1");
        assert_eq!(expr.to_string(), "1");
    }

    #[test]
    fn parse_large_single_number() {
        let expr = Expression::from_str("12345");
        assert_eq!(expr.to_string(), "12345");
    }

    #[test]
    fn parse_multiplication_precedence_over_addition() {
        let expr = Expression::from_str("1 + 2 * 3");
        assert_eq!(expr.to_string(), "(+ 1 (* 2 3))");
    }

    #[test]
    fn parse_left_associative_multiplication() {
        let expr = Expression::from_str("1 * 2 * 3");
        assert_eq!(expr.to_string(), "(* (* 1 2) 3)");
    }

    #[test]
    fn parse_left_associative_addition() {
        let expr = Expression::from_str("1 + 2 + 3");
        assert_eq!(expr.to_string(), "(+ (+ 1 2) 3)");
    }

    #[test]
    fn parse_left_associative_subtraction() {
        let expr = Expression::from_str("10 - 5 - 2");
        assert_eq!(expr.to_string(), "(- (- 10 5) 2)");
    }

    #[test]
    fn parse_left_associative_division() {
        let expr = Expression::from_str("20 / 4 / 2");
        assert_eq!(expr.to_string(), "(/ (/ 20 4) 2)");
    }

    #[test]
    fn parse_left_associative_exponentiation() {
        let expr = Expression::from_str("2 ^ 3 ^ 4");
        assert_eq!(expr.to_string(), "(^ (^ 2 3) 4)");
    }

    #[test]
    fn parse_complex_precedence_with_multiple_operations() {
        let expr = Expression::from_str("22 + 33 * 2 * 44 + 1 / 4");
        assert_eq!(expr.to_string(), "(+ (+ 22 (* (* 33 2) 44)) (/ 1 4))");
    }

    #[test]
    fn parse_mixed_operations_with_precedence() {
        let expr = Expression::from_str("2 + 2 * 5 - 3 / 5 + 5 - 3");
        assert_eq!(expr.to_string(), "(- (+ (- (+ 2 (* 2 5)) (/ 3 5)) 5) 3)");
    }

    #[test]
    fn parse_parentheses_override_precedence() {
        let expr = Expression::from_str("(2 + 444) * 5");
        assert_eq!(expr.to_string(), "(* (+ 2 444) 5)");
    }

    #[test]
    fn parse_nested_parentheses() {
        let expr = Expression::from_str("(((11)))");
        assert_eq!(expr.to_string(), "11");
    }

    #[test]
    fn parse_complex_expression_with_all_operators() {
        let expr = Expression::from_str("13 + 5 * 211 - 8 / 4");
        assert_eq!(expr.to_string(), "(- (+ 13 (* 5 211)) (/ 8 4))");
    }

    #[test]
    fn parse_exponentiation_with_other_operations() {
        let expr = Expression::from_str("2 + 3 ^ 2 * 4");
        assert_eq!(expr.to_string(), "(+ 2 (* (^ 3 2) 4))");
    }

    #[test]
    fn parse_complex_parenthetical_expression() {
        let expr = Expression::from_str("(1 + 2) * (3 + 4) / (5 - 3)");
        assert_eq!(expr.to_string(), "(/ (* (+ 1 2) (+ 3 4)) (- 5 3))");
    }

    #[test]
    fn parse_deeply_nested_parentheses() {
        let expr = Expression::from_str("((1 + 2) * (3 + (4 * 5)))");
        assert_eq!(expr.to_string(), "(* (+ 1 2) (+ 3 (* 4 5)))");
    }

    // ===== EVALUATION TESTS =====

    #[test]
    fn evaluate_single_number() {
        let expr = Expression::from_str("42");
        assert_eq!(expr.eval_no_vars(), 42);
    }

    #[test]
    fn evaluate_simple_addition() {
        let expr = Expression::from_str("2 + 3");
        assert_eq!(expr.eval_no_vars(), 5);
    }

    #[test]
    fn evaluate_simple_subtraction() {
        let expr = Expression::from_str("10 - 4");
        assert_eq!(expr.eval_no_vars(), 6);
    }

    #[test]
    fn evaluate_simple_multiplication() {
        let expr = Expression::from_str("6 * 7");
        assert_eq!(expr.eval_no_vars(), 42);
    }

    #[test]
    fn evaluate_simple_division() {
        let expr = Expression::from_str("15 / 3");
        assert_eq!(expr.eval_no_vars(), 5);
    }

    #[test]
    fn evaluate_simple_exponentiation() {
        let expr = Expression::from_str("2 ^ 3");
        assert_eq!(expr.eval_no_vars(), 8);
    }

    #[test]
    fn evaluate_precedence_multiplication_over_addition() {
        let expr = Expression::from_str("2 + 3 * 4");
        assert_eq!(expr.eval_no_vars(), 14); // 2 + (3 * 4) = 2 + 12 = 14
    }

    #[test]
    fn evaluate_precedence_exponentiation_over_multiplication() {
        let expr = Expression::from_str("2 * 3 ^ 2");
        assert_eq!(expr.eval_no_vars(), 18); // 2 * (3 ^ 2) = 2 * 9 = 18
    }

    #[test]
    fn evaluate_left_associative_subtraction() {
        let expr = Expression::from_str("10 - 3 - 2");
        assert_eq!(expr.eval_no_vars(), 5); // (10 - 3) - 2 = 7 - 2 = 5
    }

    #[test]
    fn evaluate_left_associative_division() {
        let expr = Expression::from_str("20 / 4 / 2");
        assert_eq!(expr.eval_no_vars(), 2); // (20 / 4) / 2 = 5 / 2 = 2 (integer division)
    }

    #[test]
    fn evaluate_parentheses_override_precedence() {
        let expr = Expression::from_str("(2 + 3) * 4");
        assert_eq!(expr.eval_no_vars(), 20); // (2 + 3) * 4 = 5 * 4 = 20
    }

    #[test]
    fn evaluate_complex_expression() {
        let expr = Expression::from_str("2 + 3 * 4 - 6 / 2");
        assert_eq!(expr.eval_no_vars(), 11); // 2 + (3 * 4) - (6 / 2) = 2 + 12 - 3 = 11
    }

    #[test]
    fn evaluate_nested_parentheses() {
        let expr = Expression::from_str("((2 + 3) * (4 + 1))");
        assert_eq!(expr.eval_no_vars(), 25); // (5 * 5) = 25
    }

    #[test]
    fn evaluate_zero_exponent() {
        let expr = Expression::from_str("5 ^ 0");
        assert_eq!(expr.eval_no_vars(), 1); // Any number to the power of 0 is 1
    }

    #[test]
    fn evaluate_one_exponent() {
        let expr = Expression::from_str("42 ^ 1");
        assert_eq!(expr.eval_no_vars(), 42); // Any number to the power of 1 is itself
    }

    // ===== ERROR HANDLING TESTS =====

    #[test]
    #[should_panic(expected = "Negative exponent not supported")]
    fn evaluate_negative_exponent_panics() {
        let expr = Expression::from_str("2 ^ (0 - 1)");
        expr.eval_no_vars(); // Should panic on negative exponent
    }

    #[test]
    #[should_panic(expected = "left_expr unexpected token")]
    fn parse_invalid_starting_token_panics() {
        Expression::from_str("+ 1 2");
    }

    #[test]
    #[should_panic(expected = "op unexpected token")]
    fn parse_invalid_operator_position_panics() {
        Expression::from_str("1 2 + 3");
    }

    // ===== EDGE CASES =====

    #[test]
    fn parse_expression_with_trailing_whitespace() {
        let expr = Expression::from_str("1 + 2   ");
        assert_eq!(expr.eval_no_vars(), 3);
    }

    #[test]
    fn parse_expression_with_leading_whitespace() {
        let expr = Expression::from_str("   1 + 2");
        assert_eq!(expr.eval_no_vars(), 3);
    }

    #[test]
    fn evaluate_large_numbers() {
        let expr = Expression::from_str("999 + 1");
        assert_eq!(expr.eval_no_vars(), 1000);
    }

    #[test]
    fn evaluate_division_with_integer_result() {
        let expr = Expression::from_str("9 / 3");
        assert_eq!(expr.eval_no_vars(), 3);
    }

    #[test]
    fn evaluate_division_with_truncation() {
        let expr = Expression::from_str("7 / 2");
        assert_eq!(expr.eval_no_vars(), 3); // Integer division truncates
    }

    // ===== VARIABLE TESTS =====

    #[test]
    fn evaluate_single_variable() {
        let expr = Expression::from_str("x");
        let mut vars = HashMap::new();
        vars.insert('x', 42);
        assert_eq!(expr.eval(&vars), 42);
    }

    #[test]
    fn evaluate_variable_in_expression() {
        let expr = Expression::from_str("x + 10");
        let mut vars = HashMap::new();
        vars.insert('x', 5);
        assert_eq!(expr.eval(&vars), 15);
    }

    #[test]
    fn evaluate_multiple_variables() {
        let expr = Expression::from_str("x * y + z");
        let mut vars = HashMap::new();
        vars.insert('x', 3);
        vars.insert('y', 4);
        vars.insert('z', 2);
        assert_eq!(expr.eval(&vars), 14); // 3 * 4 + 2 = 14
    }

    #[test]
    fn evaluate_assignment_expression() {
        let expr = Expression::from_str("x = 5 + 3");
        if let Some((var_name, value_expr)) = expr.is_asign() {
            assert_eq!(var_name, 'x');
            assert_eq!(value_expr.eval_no_vars(), 8);
        } else {
            panic!("Expected assignment expression");
        }
    }

    #[test]
    #[should_panic(expected = "Variable not found")]
    fn evaluate_undefined_variable_panics() {
        let expr = Expression::from_str("x + 1");
        let vars = HashMap::new(); // Empty variables map
        expr.eval(&vars); // Should panic
    }

    // ===== UNARY MINUS TESTS =====

    #[test]
    fn evaluate_unary_minus() {
        let expr = Expression::from_str("-(5)");
        assert_eq!(expr.eval_no_vars(), -5);
    }

    #[test]
    fn evaluate_unary_minus_with_expression() {
        let expr = Expression::from_str("-(2 + 3)");
        assert_eq!(expr.eval_no_vars(), -5);
    }

    #[test]
    fn evaluate_double_unary_minus() {
        let expr = Expression::from_str("-(-5)");
        assert_eq!(expr.eval_no_vars(), 5);
    }

    // ===== ADDITIONAL EDGE CASE TESTS =====

    #[test]
    fn tokenize_single_digit() {
        let lexer = Lexer::new("0");
        assert_eq!(lexer.tokens[0], Token::Number(0));
    }

    #[test]
    fn evaluate_division_by_one() {
        let expr = Expression::from_str("42 / 1");
        assert_eq!(expr.eval_no_vars(), 42);
    }

    #[test]
    fn evaluate_multiplication_by_zero() {
        let expr = Expression::from_str("999 * 0");
        assert_eq!(expr.eval_no_vars(), 0);
    }

    #[test]
    fn evaluate_addition_with_zero() {
        let expr = Expression::from_str("42 + 0");
        assert_eq!(expr.eval_no_vars(), 42);
    }

    #[test]
    fn evaluate_subtraction_with_zero() {
        let expr = Expression::from_str("42 - 0");
        assert_eq!(expr.eval_no_vars(), 42);
    }

    #[test]
    fn parse_variable_names() {
        let expr = Expression::from_str("a + B + z");
        assert_eq!(expr.to_string(), "(+ (+ a B) z)");
    }

    #[test]
    #[should_panic(expected = "Unknown operator")]
    fn evaluate_unknown_operator_panics() {
        // This would require manually creating an invalid operator expression
        // since the parser doesn't allow unknown operators
        let invalid_expr = Expression::Operator('%', vec![
            Expression::Number(5),
            Expression::Number(3),
        ]);
        invalid_expr.eval_no_vars();
    }
}
