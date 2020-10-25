#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[32] = {0}; 
    i8 tx[4096] = {0};
    i8 tx_ctor[4096] = {0};
    int sz = 0;
    long offset = 0, length = 0;
    abi_ERC20Basic_constructor((i8*)tx_ctor, &sz);
    ERC20Basic_constructor((i8*)tx_ctor, sz, &offset, &length, (i8*)storage, caller);

    abi_ERC20Basic_totalSupply((i8*)tx, &sz);
    ERC20Basic_runtime((i8*)tx, sz, &offset, &length, (i8*)storage, caller);
    prt(mem+offset);
}