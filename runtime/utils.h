#ifndef UTILS_H
#define UTILS_H
typedef unsigned char i8;

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
void inplace_reverse(i8* str, uint16_t len);
i8* pad_int(i8* out, int x);

int cmp(i8* a, i8* b);
void cpy(i8* a, i8* b);
void prt(i8* a);
void swap_endianness(i8* i);

#endif