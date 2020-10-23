#include "rt.h"
#include "contracts.h"

int main() {
    long offset = 0, length = 0;

    i8 tx[1024] = {0};
    i8 tx2[1024] = {0};
    long sz = 0; 
    long sz2 = 0;
    abi_get((i8*)tx2, &sz2);
    i8 num[32] = {0};
    abi_set((i8*)tx, &sz, pad_int((i8*)num, 1));

    SimpleStorage_constructor(NULL, 0, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx2, sz2, &offset, &length, (i8*)storage);
    prt(mem+offset); printf("\n");
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    prt(storage+32);
}