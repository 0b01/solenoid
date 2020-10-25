#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[32] = {0}; 
    i8 tx[1024] = {0};
    i8 tx_ctor[4096] = {0};
    int sz = 0;
    long offset = 0, length = 0;
    abi_ERC20Basic_constructor((i8*)tx_ctor, &sz);
    ERC20Basic_constructor((i8*)tx_ctor, sz, &offset, &length, (i8*)storage, caller);

    abi_ERC20Basic_totalSupply((i8*)tx, &sz);
    ERC20Basic_runtime((i8*)tx, sz, &offset, &length, (i8*)storage, caller);
    prt(mem+offset); printf("\n");

    i8 amt[32] = {0}; pad_int((i8*)amt, 1);
    i8 addr_b[32] = {0xA};
    i8 tx_transfer[1024] = {0}; sz = 0;
    abi_ERC20Basic_transfer(tx_transfer, &sz, addr_b, amt);
    ERC20Basic_runtime((i8*)tx_transfer, sz, &offset, &length, (i8*)storage, caller);
    prt(mem+offset); printf("\n");

    // offset = 0; length = 0;
    // i8 tx_bal[1024] = {0};
    // abi_ERC20Basic_balanceOf(tx_bal, &sz, addr_b);
    // ERC20Basic_runtime((i8*)tx_bal, sz, &offset, &length, (i8*)storage, caller);
    // prt(mem+offset); printf("\n");

    return 0;
}