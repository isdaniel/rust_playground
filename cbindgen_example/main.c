#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include "rust_lib.h"  // Make sure this is correctly generated



int main() {
    printf("3 + 5 = %d\n", add(3, 5));

    uint32_t num_ptr;
    if (string_to_uint32("123", &num_ptr)) {
        printf("string_to_uint32 success\n num_ptr = %d\n", num_ptr);
    } else {
        printf("string_to_uint32 failed\n");
    }

    getchar();
    return 0;
}