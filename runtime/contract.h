#include "utils.h"

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
void abi_set(i8* tx, int* tx_len, i8* x);
