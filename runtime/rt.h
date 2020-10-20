#ifndef RT_H
#define RT_H
#include "utils.h"

static int occupancy = 1;
i8 storage[1024*64];
extern long sp;
extern long pc;
extern i8 stack[];
extern i8 mem[];

void sload(i8* st, i8* key);
void sstore(i8* st, i8* key, i8* val);
void dump_storage();
void dump_stack(i8* label);

void udiv256(i8*, i8*, i8*);
void sdiv256(i8*, i8*, i8*);
void neg(i8*);
void powmod(i8*, i8*, i8*);
void SimpleStorage_constructor(
    i8* tx,
    long tx_sz,
    long* ret_offset,
    long* ret_len,
    i8* storage
);
void SimpleStorage_runtime(
    i8* tx,
    long tx_sz,
    long* ret_offset,
    long* ret_len,
    i8* storage
);
void abi_set_0(i8* tx, int* tx_len, i8* x);


#endif