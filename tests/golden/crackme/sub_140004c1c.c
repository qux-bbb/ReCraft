// L1 structure recovery (4 blocks)
void sub_140004c1c_l1(void) {
    rax = *(u64*)(rcx);
    rbx = rcx;
    rbp = *(u64*)((rcx + 8));
    rsi = *(u64*)(rax);
    sub_14000af74(rsi);
    rdx = *(u64*)((rbx + 8));
    r14b = al;
    r9 = *(u64*)(rbx);
    r8 = *(u64*)((rbx + 0xADDR));
    rcx = *(u64*)((rbx + 0xADDR));
    var_20 = rdx;
    r9 = *(u64*)(r9);
    r8 = *(u64*)(r8);
    rcx = *(u64*)(rcx);
    sub_14000ad14(rcx, 1, r8, r9, *(u64*)((rbx + 8)));
    rcx = *(u64*)((rbx + 0xADDR));
    if (rax == *(u64*)(rcx)) {
        rdx = *(u64*)(rbx);
        r8 = *(u64*)((rbx + 8));
        rdx = *(u64*)(rdx);
        sub_14000acfc((rdi + 0xADDR), rdx);
    }
    r8 = rbp;
    sub_14000b03c(r14b, rsi);
    eax = 0;
    return;
}
