#include <stdint.h>
typedef unsigned char i8;
extern void udiv256(i8* n, i8* d, i8* q);
extern void keccak256(const i8*msg, uint16_t size, i8*result);
int main() {
   // i8 a[32] = {0x00, 0xAA};
   // i8 b[32] = {0x01};
   // i8 c[32] = {0};

   // udiv256(a, b, c);

   // for (int i = 0 ; i < 32; i++ ) printf("%02X", c[i]);

   i8 msg[64] = {0};
   memset(msg+12, 0xBB ,20);
   i8 out[32];
   keccak256(msg, 64, out);
   
   prt(out);
}
