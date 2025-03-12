#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include "rust_lib.h"  // Make sure this is correctly generated



int main() {
    // uint32_t num_ptr;
    // if (string_to_uint32("123", &num_ptr)) {
    //     printf("string_to_uint32 success\n num_ptr = %d\n", num_ptr);
    // } else {
    //     printf("string_to_uint32 failed\n");
    // }
    
    // int32_t num_intPtr;
    // if (string_to_int32("-123", &num_intPtr)) {
    //     printf("string_to_int32 success\n num_intPtr = %d\n", num_intPtr);
    // } else {
    //     printf("string_to_int32 failed\n");
    // }
    
    // const char* str1 = get_helloWorld();
    // const char* str2 = get_helloWorld();
    // printf("%s, pos:{%p}\r\n",str1,str1);
    // printf("%s, pos:{%p}\r\n",str2,str2);
    
    Person* daniel = create_person(1,"Daniel");
    if(daniel == NULL){
        printf("Failed to create person.\n");
        return 1;
    }
    printf("Person Name: %s\n", daniel->name);
    free_person(daniel);
    getchar();
    return 0;
}