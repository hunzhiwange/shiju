#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include "sha1.h"
int main(void) {
  _Alignas(4) char input[81];
  memset(input, 'a', sizeof(input));
  SHA1_CTX ctx;
  unsigned char digest[20];
  SHA1DCInit(&ctx);
  SHA1DCUpdate(&ctx, input + 1, 80);
  if (SHA1DCFinal(digest, &ctx)) return 2;
  for (int i = 0; i < 20; i++) printf("%02x", digest[i]);
  puts("");
  return 0;
}
