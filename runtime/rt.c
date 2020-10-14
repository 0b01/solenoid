#include <stdio.h>
#include "utils.h"

extern long sp;
extern long pc;
extern unsigned char stack[];
extern unsigned char mem[];
extern void contract_constructor(char*, long, long*, long*, char*);
extern void contract_runtime(char*, long, long*, long*, char*);

int occupancy = 1;
unsigned char storage[1024*64];

void dump_storage() {
    for (int i = 0; i < occupancy * 64; i += 64) {
        for (int j = i + 31; j >= i; j--) {
            unsigned char k = storage[j];
            printf("%02X", k);
        }
        printf(" : ");
        for (int j = i + 63; j >= i+32; j--) {
            unsigned char k = storage[j];
            printf("%02X", k);
        }
        printf("\n");
    }
    printf("\n");
}


void dump_stack(char* label) {
    printf("----%s----\nstack:(%ld)@%ld\n", label, sp, pc);
    int top = 10;
    int size = top > 0 ? top * 32 : (1024 * 256 / 8);
    for (int i = 0; i < size; i += 32) {
        char* arrow = (sp * 32) == i ? " ->" : "   ";
        printf("%s@%04x ", arrow, i);
        if ((sp * 32) == i) break;
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


void cpy(char* a, char* b) {
    for (int i = 0; i < 32; i++) {
        a[i] = b[i];
    }
}

void sload(char* st, char* key, char* ret) {
    // printf("sload called\n");
    for (int i = 0; i < 1024 * 64; i += 64) {
        if (cmp(st + i, key)) {
            cpy(ret, st+i+32);
            break;
        }
    }
}

void sstore(char* st, char* key, char* val) {
    // printf("sstore called\n");
    if (occupancy == 1024) { return; }

    int found = 0;
    int loc = occupancy * 64;

    for (int i = 0; i < 1024 * 64; i += 64) {
        if (cmp(st + i, key)) {
            found = 1;
            loc = i;
            break;
        }
    }

    if (!found) {
        occupancy++;
        cpy(st + loc, key);
    }
    for (int i = 0; i < 32; i++) {
        cpy(st + loc+32, val);
    }
}

void swap_endianness(char* i) {
    inplace_reverse(i, 32);
}

int main() {
    long offset = 0, length = 0;

    contract_constructor(NULL, 0, &offset, &length, (char*)storage);
    printf("return offset: %ld\nreturn length: %ld\n", offset, length);
    printf("storage occupancy: %d\n", occupancy);
    dump_storage();

    unsigned char tx[36] = {0x60, 0xfe, 0x47, 0xb1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a};
    contract_runtime((char*)tx, sizeof(tx), &offset, &length, (char*)storage);
    printf("return offset: %ld\nreturn length: %ld\n", offset, length);
    printf("storage occupancy: %d\n", occupancy);
    dump_storage();

    contract_runtime((char*)tx, sizeof(tx), &offset, &length, (char*)storage);
    printf("return offset: %ld\nreturn length: %ld\n", offset, length);
    printf("storage occupancy: %d\n", occupancy);
    dump_storage();
}