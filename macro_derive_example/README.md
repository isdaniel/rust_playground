# Macro Example

This project demonstrates the use of a custom derive macro in Rust.

## Usage

To use the custom derive macro, add the following to your Cargo.toml file:

```toml
[dependencies]
macro_example = { path = "./macro_example" }
```

Then, in your Rust code, you can use the `HelloMacro` trait as follows:

```rust
use macro_example::HelloMacro;

#[derive(HelloMacro)]
struct MyStruct;

fn main() {
    MyStruct::hello_macro();
}
```

This will print the following message:

```
Hello World!! My name is MyStruct!
```

## Key Features

- Custom derive macro for the `HelloMacro` trait
- Prints a message with the name of the struct
