// L1 structure recovery (4 blocks)
void sub_14000b1d0_l1(void) {
    rcx = *(u64*)((rcx + 0xADDR));
    r11 = r9;
    rbx = rdx;
    if (rcx == 0) {
        eax = 0xADDR;
    } else {
        rax = *(u64*)(rcx);
        r10d = var_80;
        r9 = var_78;
        edx = var_70;
        rax = *(u64*)((rax + 0xADDR));
        var_30 = r10d;
        var_28 = r9;
        r9 = r11;
        var_20 = edx;
        rdx = rbx;
        sub_140028010();
    }
    return;
}
