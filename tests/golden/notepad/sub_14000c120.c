// L1 structure recovery (3 blocks)
void sub_14000c120_l1(void) {
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx == 0) {
        sub_1400091d4();
    }
    rax = *(u64*)(rcx);
    rax = *(u64*)((rax + 0xADDR));
    sub_140028010(rcx, (r11 + 0xADDR), (r11 + 0xADDR), (r11 + 0xADDR));
    return;
}
