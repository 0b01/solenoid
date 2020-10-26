#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[20] = {0}; 
    i8 addr_b[20] = {0}; 
    memset(caller, 0xAA, 20);
    memset(addr_b, 0xCC, 20);
    caller[19] = 0;
    addr_b[19] = 0;
    long offset = 0, length = 0;

    i8 tx_ctor[1024] = {0};
    i8 tx[1024] = {0};
    i8 tx2[1024] = {0};
    int sz_ctor = 0; 
    int sz = 0; 
    int sz2 = 0;
    abi_SimpleAddress_constructor((i8*)tx_ctor, &sz_ctor);
    SimpleAddress_constructor(tx_ctor, sz_ctor, &offset, &length, (i8*)storage, caller);

    abi_SimpleAddress_get((i8*)tx2, &sz2);
    SimpleAddress_runtime(tx2, sz2, &offset, &length, (i8*)storage, caller);
    prt(SimpleAddress_mem+offset); printf("\n");

    abi_SimpleAddress_set((i8*)tx, &sz, addr_b);
    SimpleAddress_runtime(tx, sz, &offset, &length, (i8*)storage, caller);

    abi_SimpleAddress_get((i8*)tx2, &sz2);
    SimpleAddress_runtime(tx2, sz2, &offset, &length, (i8*)storage, caller);
    prt(SimpleAddress_mem+offset); printf("\n");

    return 0;
}