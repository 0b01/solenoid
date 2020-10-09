#include <stdio.h>

extern unsigned char stack[];
extern void contract();

void dump_stack(int top) {
    int size = top > 0 ? top * 32 : (1024 * 256 / 8);
    for (int i = 0; i < size; i += 32) {
        printf("%04x ", i);
        for (int j = i + 31; j >= i; j--) {
            unsigned char k = stack[j];
            printf("%02X", k);
        }
        printf("\n");
    }
}

int main() {
    dump_stack(10);
    contract();
    printf("--\n");
    dump_stack(10);
}