#include <stdio.h>

extern long sp;
extern unsigned char stack[];
extern unsigned char mem[];
extern void contract_constructor(char*, long, long*, long*, char*);
// extern void contract_runtime(char*, long, long*, long*, char*);

int occupancy = 0;
unsigned char storage[1024*32];

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

int cmp(char* a, char* b) {
    for (int i = 0; i < 32; i++) {
        if (a[i] != b[i]) return 0;
    }
    return 1;
}

void sload(char* st, char* key, char* ret) {
    for (int i = 0; i < 1024 * 32; i += 64) {
        if (cmp(st + i, key)) {
            for (int j = 0; j < 32; j++) {
                ret[j] = st[i+j+32];
            }
        }
    }
}

void sstore(char* st, char* key, char* val) {

    if (occupancy == 1024) { return; }

    for (int i = 0; i < 32; i++) {
        st[64*occupancy+i] = key[i];
    }
    for (int i = 0; i < 32; i++) {
        st[64*occupancy+i+32] = val[i];
    }
    occupancy++;
}

int main() {

    long offset = 0, length = 0;
    contract_constructor(NULL, 0, &offset, &length, (char*)storage);
    printf("%ld\n%ld\n", offset, length);

    // unsigned char tx[36] = {0x60, 0xfe, 0x47, 0xb1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a};
    // contract_runtime((char*)tx, sizeof(tx), &offset, &length, storage);
    // printf("%ld\n%ld\n", offset, length);
}