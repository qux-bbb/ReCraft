// L1 structure recovery (3 blocks)
void sub_140023d18_l1(void) {
    rdx = rcx;
    eax = 0;
    rcx = qword_ADDR;
    if (rcx != 0) {
        xmm0 = xmmword_ADDR;
        r8 = &var_40;
        var_28 = 1;
        var_20 = r8;
        r9d = 0;
        var_40 = xmm0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, 0xADDR);
    }
    return;
}
