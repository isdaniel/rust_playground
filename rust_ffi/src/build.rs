extern crate cc;

fn main() {
    cc::Build::new()
        .file("c-lib/hello.c")  // Update this path to your C file location
        .compile("my_lab");     // This will produce `my_lab.lib` on Windows
}
