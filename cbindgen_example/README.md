## How to execute

```
#ensure install cbindgen
cargo install cbindgen

#build .h file
cbindgen --config cbindgen.toml --crate cbindgen-example --output rust_lib.h

gcc main.c -o main -L./target/release -lcbindgen_example
```

