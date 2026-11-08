// L1 structure recovery (3 blocks)
void branchy_l1(void) {
    if (edi <= 0 /* signed */) {
        __asm("setne al");
        eax = (uint32)(al);
        eax += eax;
        return;
    }
    eax = 0;
    al = (uint8)((edi >= 0xADDR /* signed */));
    eax = ((rax + rax) + 1);
    return;
}
