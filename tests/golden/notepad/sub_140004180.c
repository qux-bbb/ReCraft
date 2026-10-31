// L1 structure recovery (4 blocks)
void sub_140004180_l1(void) {
    rax = *(u64*)((rcx + 0xADDR));
    rbx = rcx;
    r10 = *(u64*)((rcx + 8));
    rax = *(u64*)(rax);
    if (*(u64*)(r10) < rax) {
        rcx = *(u64*)((rbx + 8));
        edx = 1;
        *(u64*)(rcx) = (*(u64*)(rcx) + rdx);
    } else {
        rax = var_68;
        r9 = *(u64*)(r9);
        r8 = *(u64*)(r8);
        rdx = *(u64*)(rdx);
        r11d = *(u32*)(rax);
        rax = var_60;
        rcx = *(u64*)((rcx + 0xADDR));
        var_28 = r11d;
        r10 = *(u64*)(rax);
        var_20 = r10;
        sub_140007004(rcx, rdx, r8, r9, *(u64*)(rax), *(u32*)(rax));
        dl = 0;
        if (al != 0) {
            rcx = *(u64*)((rbx + 8));
            edx = 1;
            *(u64*)(rcx) = (*(u64*)(rcx) + rdx);
            al = dl;
            return;
        }
    }
    al = dl;
    return;
}
