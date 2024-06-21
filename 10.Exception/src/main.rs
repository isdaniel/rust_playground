use std::io::Write;
use std::{fs::File, io::Read};
use std::error::Error;

// fn read_username_from_file() -> Result<String, std::io::Error>{
//     let mut f = File::open("hello.txt")?;
//     //as same as below.
//     // let mut f = match File::open("hello.txt"){
//     //     Ok(file) => file,
//     //     Err(e) => return Err(e),
//     // };
//     let mut s = String::new(); 
//     f.read_to_string(&mut s)?;
//     Ok(s)
// }

fn read_username_from_file() -> Result<String, std::io::Error>{
    let mut s = String::new(); 
    File::open("hello.txt")?.read_to_string(&mut s)?;
    Ok(s)
}

fn main() -> Result<(),Box<dyn Error>>{
    //panic!("server crash!!");
    // let v = vec![1,2,3];
    // v[99];

    //Result enum
    let f = File::open("hello.txt");
    let f = match f{
        Ok(file) => file,
        Err(error) => match error.kind(){
            std::io::ErrorKind::NotFound => match File::create("hello.txt"){
                Ok(fc) => fc,
                Err(e) => panic!("Problem creating the file: {:?}", e),
            },
            other_error => panic!("Problem opening the file: {:?}", other_error),
        }
    };

    let f = File::open("hello.txt").unwrap_or_else(|error|{
        if error.kind() == std::io::ErrorKind::NotFound{
            File::create("hello.txt").unwrap_or_else(|error|{
                panic!("Problem creating the file: {:?}", error);
            })
        }else{
            panic!("Problem opening the file: {:?}", error);
        }
    });
    
    

    let mut f = File::options().read(true).write(true).create(true).open("hello.txt").expect("Failed to open hello.txt");

    f.write(b"hello world, create")?;

    let result = read_username_from_file();
    match result{
        Ok(s) => println!("read from file: {}",s),
        Err(e) => println!("Error: {}",e),
    }
    Ok(())
}
