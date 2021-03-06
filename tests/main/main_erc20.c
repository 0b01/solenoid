#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[20] = {0xA}; 
    i8 addr_b[20] = {0};
    memset(addr_b, 0xBB, 20);
    int sz = 0;

    i8 tx_ctor[4096] = {0};
    long offset = 0, length = 0;
    abi_ERC20Basic_constructor(tx_ctor, &sz);
    ERC20Basic_constructor(tx_ctor, sz, &offset, &length, storage, caller);

    i8 tx_supply[1024] = {0};
    abi_ERC20Basic_totalSupply(tx_supply, &sz);
    ERC20Basic_runtime(tx_supply, sz, &offset, &length, storage, caller);
    prt(ERC20Basic_mem+offset); printf("\n");

    i8 amt[32] = {0}; pad_int(amt, 0x1);
    i8 tx_transfer[1024] = {0}; int sz_transfer = 0;
    abi_ERC20Basic_transfer(tx_transfer, &sz_transfer, addr_b, amt);
    offset = length = 0;
    ERC20Basic_runtime(tx_transfer, sz_transfer, &offset, &length, storage, caller);

    offset = 0; length = 0;
    i8 tx_bal[1024] = {0};
    abi_ERC20Basic_balanceOf(tx_bal, &sz, addr_b);
    ERC20Basic_runtime(tx_bal, sz, &offset, &length, storage, addr_b);
    prt(ERC20Basic_mem+offset); printf("\n");

    offset = 0; length = 0;
    abi_ERC20Basic_balanceOf(tx_bal, &sz, caller);
    ERC20Basic_runtime(tx_bal, sz, &offset, &length, storage, addr_b);
    prt(ERC20Basic_mem+offset); printf("\n");

    return 0;
}
