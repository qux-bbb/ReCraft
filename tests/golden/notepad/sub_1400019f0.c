// L1 structure recovery (5 blocks)
void sub_1400019f0_l1(void) {
    if (rcx != qword_ADDR) {
        return sub_140001cf0();
    }
    rcx = rol(rcx, 0xADDR);
    if ((cx & 0xADDR) != 0) {
        rcx = ror(rcx, 0xADDR);
    }
    return;
}
