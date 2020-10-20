#include "utils.h"
#include "rt.h"

/* overwrite key */
void sload(i8* st, i8* key) {
    // printf("sload called\n");
    for (int i = 0; i < 1024 * 64; i += 64) {
        if (cmp(st + i, key)) {
            cpy(key, st+i+32);
            break;
        }
    }
}

void sstore(i8* st, i8* key, i8* val) {
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

void dump_storage() {
    for (int i = 0; i < occupancy * 64; i += 64) {
        for (int j = i + 31; j >= i; j--) {
            i8 k = storage[j];
            printf("%02X", k);
        }
        printf(" : ");
        for (int j = i + 63; j >= i+32; j--) {
            i8 k = storage[j];
            printf("%02X", k);
        }
        printf("\n");
    }
    printf("\n");
}

void dump_stack(i8* label) {
    printf("----%s----\nstack:(%ld)@%ld\n", label, sp, pc);
    int top = 10;
    int size = top > 0 ? top * 32 : (1024 * 256 / 8);
    for (int i = 0; i < size; i += 32) {
        i8* arrow = (sp * 32) == i ? (i8*)" ->" : (i8*)"   ";
        printf("%s@%04x ", arrow, i);
        if ((sp * 32) == i) break;
        for (int j = i + 31; j >= i; j--) {
            i8 k = stack[j];
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
            i8 k = mem[j];
            printf("%02X", k);
        }
        printf("\n");
    }
    printf("\n");
}
