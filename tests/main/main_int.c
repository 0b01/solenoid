#include "../../runtime/rt.h"

extern void test_constructor(i8*, long, long*, long*, i8*, i8*);
extern long test_sp;
extern long test_pc;
extern i8 test_stack[];
extern i8 test_mem[];
int main() {
    i8 caller[32] = {0}; 
    long offset = 0;
    long length = 0;
    test_constructor(NULL, 0, &offset, &length, (i8*)storage, caller);

    for(int i = 0; i < test_sp; i++) {
        prt(test_stack + i * 32);
        printf("\n");
    }
}