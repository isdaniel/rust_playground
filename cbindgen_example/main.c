#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include "rust_lib.h"  // Make sure this is correctly generated

void print_hashset_contain(IntHashSet* hashset, int val){
    if(int_set_contain(hashset, val)){
        printf("contain %d\r\n",val);
    } else {
        printf("didn't contain %d\r\n",val);
    }
}

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
    
    // Person* daniel = create_person(1,"Daniel");
    // if(daniel == NULL){
    //     printf("Failed to create person.\n");
    //     return 1;
    // }

    // printf("Person Name: %s\n", daniel->name);
    // printf("Person Json: %s\n",  serialize_person(daniel));
    // free_person(daniel);
    
    IntHashSet* hashset = int_set_new();
    int_set_insert(hashset,1);
    print_hashset_contain(hashset,1);
    print_hashset_contain(hashset,2);
    int_set_free(hashset);
    
    getchar();
    return 0;
}