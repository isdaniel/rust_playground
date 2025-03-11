# Set variables for paths
C_OUTPUT := main
RUST_LIB := libcbindgen_example.a
LIB_DIR := ./target/release

# Set the compiler and flags
CC := gcc
CFLAGS := -Wall -Wextra
LDFLAGS := -L$(LIB_DIR) -lcbindgen_example

$(C_OUTPUT): main.c $(LIB_DIR)/$(RUST_LIB)
	$(CC) $(CFLAGS) -o $(C_OUTPUT) main.c $(LDFLAGS)

$(LIB_DIR)/$(RUST_LIB): 
	cargo build --release

clean:
	rm -f $(C_OUTPUT)
	rm -f $(LIB_DIR)/$(RUST_LIB)

rebuild: clean $(C_OUTPUT)

all: $(C_OUTPUT)
