#include "rt.h"
#include "../out/src/contracts.h"

int main() {
    long offset = 0, length = 0;

    SimpleStorage_constructor(NULL, 0, &offset, &length, (i8*)storage);
    printf("return offset: %ld\nreturn length: %ld\n", offset, length);
    printf("storage occupancy: %d\n", occupancy);
    dump_storage();
    offset = 0; length = 0;

    i8 tx[1024] = {0};
    int sz = 0;
    i8 num[32] = {0};
    abi_set((i8*)tx, &sz, pad_int((i8*)num, 1));
    printf("%d\n", sz);

    for (int i = 0; i < 10; i++) {
        SimpleStorage_runtime(tx, sz, &offset, &length, (i8*)storage);
        printf("return offset: %ld\nreturn length: %ld\n", offset, length);
        printf("storage occupancy: %d\n", occupancy);
        dump_storage();
        offset = 0; length = 0;
    }
}