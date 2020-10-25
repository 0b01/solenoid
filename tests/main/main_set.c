#include "rt.h"
#include "contracts.h"

int main() {
    long offset = 0, length = 0;

    i8 tx_ctor[1024] = {0};
    i8 tx[1024] = {0};
    i8 tx2[1024] = {0};
    int sz_ctor = 0; 
    int sz = 0; 
    int sz2 = 0;
    abi_SimpleStorage_constructor((i8*)tx_ctor, &sz_ctor);
    abi_SimpleStorage_get((i8*)tx2, &sz2);
    i8 num[32] = {0};
    abi_SimpleStorage_set((i8*)tx, &sz, pad_int((i8*)num, 1));

    SimpleStorage_constructor(tx_ctor, sz_ctor, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx2, sz2, &offset, &length, (i8*)storage);
    prt(mem+offset); printf("\n");
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
    prt(storage+32);
}