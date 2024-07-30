use std::{fmt::Display, error::Error};

fn main() {
    let res =match add(0,5){
        Ok(val) => val,
        Err(e) => {
            panic!("Error: {e}");
        }
    };

    dbg!(res);
}

#[derive(Debug)]
enum CustomerError{
    CannotBeZero
}

impl Error for CustomerError{}

impl Display for CustomerError{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            CustomerError::CannotBeZero => "Number cannot be zero"
        };

        write!(f, "Error: {msg}")
    }    
}


fn add(a:i32, b: i32) -> Result<i32,CustomerError>{
    if a == 0 || b == 0 {
        return Err(CustomerError::CannotBeZero);
    }

    Ok(a+b)
}
