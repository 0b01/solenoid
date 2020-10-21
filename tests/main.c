#include "../runtime/rt.h"

extern void test_constructor(i8*, long, long*, long*, i8*);
int main() {
    long offset = 0;
    long length = 0;
    test_constructor(NULL, 0, &offset, &length, (i8*)storage);

    for(int i = 0; i < sp; i++) {
        prt(stack + i * 32);
        printf("\n");
    }
}