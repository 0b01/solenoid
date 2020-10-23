#include "rt.h"
#include "contracts.h"

int main() {
    i8 tx[1024] = {0};
    long sz = 0;
    long offset = 0, length = 0;

    abi_constructor(tx, &sz, 1);
    flipper_constructor(tx, sz, &offset, &length, (i8*)storage);
}