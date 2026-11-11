// L1 structure recovery (3 blocks)
void branchy_l1(void) {
    eax = var_4;
    if (eax <= 0 /* signed */) {
        __asm("setne al");
        eax = (uint32)(al);
        eax += eax;
        return;
    }
    al = (uint8)((eax >= 0xADDR /* signed */));
    eax = (uint32)(al);
    eax = ((eax + eax) + 1);
    return;
}
