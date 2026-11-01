// L1 structure recovery (4 blocks)
void sub_14000ce80_l1(void) {
    if (dword_ADDR == 2 && qword_ADDR != 0) {
        r9d = 0;
        PostMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
    }
    return;
}
