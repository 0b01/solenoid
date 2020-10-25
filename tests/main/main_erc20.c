#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[32] = {0xA}; 
    i8 addr_b[32] = {0xB};
    int sz = 0;

    i8 tx_ctor[4096] = {0};
    long offset = 0, length = 0;
    abi_ERC20Basic_constructor(tx_ctor, &sz);
    ERC20Basic_constructor(tx_ctor, sz, &offset, &length, storage, caller);

    i8 tx_supply[1024] = {0};
    abi_ERC20Basic_totalSupply(tx_supply, &sz);
    ERC20Basic_runtime(tx_supply, sz, &offset, &length, storage, caller);
    // prt(mem+offset); printf("\n");

    i8 amt[32] = {0}; pad_int(amt, 0xC);
    i8 tx_transfer[1024] = {0};
    abi_ERC20Basic_transfer(tx_transfer, &sz, addr_b, amt);
    // for (int i=0; i < 68; i++) printf("%02x", tx_transfer[i]);
    ERC20Basic_runtime(tx_transfer, sz, &offset, &length, storage, caller);
    // prt(mem+offset); printf("\n");

    // offset = 0; length = 0;
    // i8 tx_bal[1024] = {0};
    // abi_ERC20Basic_balanceOf(tx_bal, &sz, addr_b);
    // ERC20Basic_runtime(tx_bal, sz, &offset, &length, storage, addr_b);
    // printf("%d\n%d\n", offset, length);
    // prt(mem+offset); printf("\n");

    return 0;
}