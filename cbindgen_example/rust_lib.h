#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>

typedef struct IntHashSet IntHashSet;

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

char *serialize_person(const struct Person *person);

struct Person *create_person(int id, const char *name);

void free_person(struct Person *person);

struct IntHashSet *int_set_new(void);

void int_set_insert(struct IntHashSet *set, uintptr_t value);

void int_set_remove(struct IntHashSet *set, uintptr_t value);

bool int_set_contain(struct IntHashSet *set, uintptr_t value);

void int_set_free(struct IntHashSet *set);
