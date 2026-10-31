// L1 structure recovery (3 blocks)
void sub_140002f80_l1(void) {
    if (*(u8*)(rcx) != 0) {
        rdx = (rcx + 0xADDR);
        rcx += 0xADDR;
        sub_14000647c(rcx, (rcx + 0xADDR));
    }
    return;
}
