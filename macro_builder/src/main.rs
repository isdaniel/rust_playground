use derive_builder::Builder;

#[derive(Debug, Builder)] 
pub struct TestBuilder {
    executable: String,
    args: Vec<String>,
    current_dir: String,
}

fn main() {
    let command = TestBuilder::builder()
        .executable("cargo".to_owned())
        .current_dir("./".to_owned())
        .args(["build".to_string(),"--release".to_string()].to_vec())
        .build()
        .unwrap();

    println!("Executing command: {:?}", command);
}
