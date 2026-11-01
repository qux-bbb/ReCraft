// L1 structure recovery (4 blocks)
void sub_14000ac30_l1(void) {
    rcx = *(u64*)((rcx + 0xADDR));
    r11d = r9d;
    rbx = rdx;
    if (rcx == 0) {
        eax = 0xADDR;
    } else {
        r9d = var_78;
        rax = *(u64*)(rcx);
        r10 = var_80;
        rdx = var_70;
        var_30 = r10;
        rax = *(u64*)((rax + 0xADDR));
        var_28 = r9d;
        r9d = r11d;
        var_20 = rdx;
        rdx = rbx;
        sub_140028010();
    }
    return;
}
