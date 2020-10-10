#include <stdio.h>

extern long sp;
extern unsigned char stack[];
extern unsigned char mem[];
extern void contract();

void dump_stack(char* label) {
    printf("----%s----\nstack:\n", label);
    int top = 5;
    int size = top > 0 ? top * 32 : (1024 * 256 / 8);
    for (int i = 0; i < size; i += 32) {
        char* arrow = (sp * 32) == i ? " ->" : "   ";
        printf("%s@%04x ", arrow, i);
        for (int j = i + 31; j >= i; j--) {
            unsigned char k = stack[j];
            printf("%02X", k);
        }
        printf("\n");
    }
    printf("\n");

    printf(" mem:\n");
    size = top > 0 ? top * 32 : (1024 * 256 / 8);
    for (int i = 0; i < size; i += 32) {
        printf(" %04x ", i);
        for (int j = i + 31; j >= i; j--) {
            unsigned char k = mem[j];
            printf("%02X", k);
        }
        printf("\n");
    }
    printf("\n");
}

int main() {
    contract();
}