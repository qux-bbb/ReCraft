// L1 structure recovery (6 blocks)
void sub_140024df4_l1(void) {
    eax = dword_ADDR;
    if ((eax == 0) || (ecx >= 0 /* signed */)) {
        return;
    }
    if (eax != -1) {
        return;
    }
}
