# Set variables for paths
C_OUTPUT := main
RUST_LIB := libcbindgen_example.a
LIB_DIR := ./target/release
OUTPUT_H_FILE := rust_lib.h
CBINDGEN_NAME := cbindgen-example

# Set the compiler and flags
CC := gcc
CFLAGS := -Wall -Wextra
LDFLAGS := -L$(LIB_DIR) -lcbindgen_example

$(C_OUTPUT): main.c $(LIB_DIR)/$(RUST_LIB)
	$(CC) $(CFLAGS) -o $(C_OUTPUT) main.c $(LDFLAGS)

$(LIB_DIR)/$(RUST_LIB): 
	cargo build --release

update-ffi:
	cbindgen --config cbindgen.toml --crate $(CBINDGEN_NAME) --output $(OUTPUT_H_FILE)

clean:
	rm -f $(C_OUTPUT)
	rm -f $(LIB_DIR)/$(RUST_LIB)
	rm -f $(OUTPUT_H_FILE)
	
rebuild: clean update-ffi $(C_OUTPUT)

all: $(C_OUTPUT)
