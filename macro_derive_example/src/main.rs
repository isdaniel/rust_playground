use hello_macro_derive::HelloMacro;
use macro_example::HelloMacro;

#[derive(HelloMacro)]
struct People;

fn main() {
    People::hello_macro();
}
