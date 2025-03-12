#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>

/**
 * Define a struct that will be shared with C
 */
typedef struct Person {
  int id;
  char *name;
} Person;

bool string_to_uint32(const char *str, uint32_t *number);

bool string_to_int32(const char *str, int32_t *number);

const char *get_helloWorld(void);

struct Person *create_person(int id, const char *name);

void free_person(struct Person *person);
