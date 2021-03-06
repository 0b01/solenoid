#include "rt.h"
#include "contracts.h"

long offset = 0, length = 0;
i8 tx[1024] = {0};
int sz = 0;
i8 tx2[1024] = {0};
int sz2 = 0;
i8 tx3[1024] = {0};
int sz3 = 0;

void run() {
    i8 caller[32] = {0}; 
    flipper_constructor(tx, sz, &offset, &length, (i8*)storage, caller);
    prt(storage+32); printf("\n");

    flipper_runtime(tx2, sz2, &offset, &length, (i8*)storage, caller);
    prt(storage+32); printf("\n");

    flipper_runtime(tx2, sz2, &offset, &length, (i8*)storage, caller);
    prt(storage+32); printf("\n");

    flipper_runtime(tx3, sz3, &offset, &length, (i8*)storage, caller);
    prt(storage+32); printf("\n");
    prt(flipper_mem+offset); printf("\n");

    flipper_runtime(tx2, sz2, &offset, &length, (i8*)storage, caller);
    prt(storage+32); printf("\n");
}

int main() {

    abi_flipper_constructor(tx, &sz, 1);
    abi_flipper_flip(tx2, &sz2);
    abi_flipper_get(tx3, &sz3);
    run();

    // printf("\n");

    memset(tx, 0, 1024);
    abi_flipper_constructor(tx, &sz, 0);
    run();
}
