pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn add_three(a:i32) -> i32{
    a + 3
}

pub struct Guess {
    value: i32,
}


impl Guess {
    pub fn new(value: i32) -> Guess {
        if value < 1 || value > 100 {
            panic!("Guess value must be between 1 and 100, got {}.", value);
        }

        Guess {
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use std::string;

    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn it_add_three() {
        let a = 2;
        assert_eq!(add_three(a), 5);
        //assert_ne!(add_three(a), 6);
    }

    #[test]
    #[should_panic(expected = "Guess value must be between 1 and 100")]
    fn it_should_panic(){
        Guess::new(200);
    }
    

    #[test]
    fn it_result_test() -> Result<(),String>{
        if 2 + 2 == 4 {
            Ok(())
        } else {
            Err(String::from("Two plus two does not equal four"))
        }
    }
    // #[test]
    // fn panic_test(){
    //     panic!("Make this test fail");
    // }
}
