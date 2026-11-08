// L1 structure recovery (3 blocks)
void switcher_l1(void) {
    edx = (rdi - 1);
    eax = 0xADDR;
    if (edx <= 2) {
        eax = (rdi + (rdi * 4));
        eax += eax;
    }
    return;
}
