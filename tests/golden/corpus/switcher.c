// L1 structure recovery (11 blocks)
void switcher_l1(void) {
    if ((ecx == 0)) {
        eax = 0xADDR;
        return;
    }
    ecx = (ecx - 1);
    if ((ecx == 1)) {
        eax = 0xADDR;
        return;
    }
    ecx = (ecx - 1);
    if ((ecx == 1)) {
        eax = 0xADDR;
        return;
    }
    ecx = (ecx - 1);
    if ((ecx == 1)) {
        eax = 0xADDR;
        return;
    }
    if ((ecx == 1)) {
        eax = 0xADDR;
        return;
    }
    eax = 0xADDR;
    return;
}
