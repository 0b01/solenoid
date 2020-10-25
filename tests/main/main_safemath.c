#include "rt.h"
#include "contracts.h"

int main() {
    i8 caller[32] = {0};
    i8 tx_ctor[1024] = {0};
    int sz = 0;
    long offset = 0; long length = 0;
    abi_TestSafeMath_constructor(tx_ctor, &sz);
    TestSafeMath_constructor(tx_ctor, sz, &offset, &length, storage, caller);

    i8 tx_sub[1024];
    abi_TestSafeMath_sub(tx_sub, &sz);
    TestSafeMath_runtime(tx_sub, sz, &offset, &length, storage, caller);

    offset = 0; length = 0;
    i8 tx_get[1024]; int sz_get = 0;
    abi_TestSafeMath_get(tx_get, &sz_get);
    TestSafeMath_runtime(tx_get, sz_get, &offset, &length, storage, caller);
    prt(TestSafeMath_mem+offset);

    return 0;
}
