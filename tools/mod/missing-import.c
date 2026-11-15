/* A game stand-in that imports a function KERNEL32 lacks: Windows ends it with 0xc0000139 while loading (#41). */
#include <stdio.h>
void Yo3MissingFunction(void);
int main(void) { puts("FAIL line: the missing-import fixture ran"); Yo3MissingFunction(); return 0; }
