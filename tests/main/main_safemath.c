#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[32];
    i8 tx_ctor[1024];
    int sz = 0;
    long offset = 0; long length = 0;
    abi_TestSafeMath_constructor(tx_ctor, &sz);
    TestSafeMath_constructor(tx_ctor, sz, &offset, &length, storage, caller);

    i8 tx_sub[1024];
    abi_TestSafeMath_sub(tx_sub, &sz);
    TestSafeMath_constructor(tx_sub, sz, &offset, &length, storage, caller);

    i8 tx_get[1024];
    abi_TestSafeMath_get(tx_get, &sz);
    TestSafeMath_constructor(tx_get, sz, &offset, &length, storage, caller);
    prt(mem+offset);
}
