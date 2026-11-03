// L1 structure recovery (1 blocks)
void sub_140002b88_l1(void) {
    rbx = *(u64*)((r9 + 8));
    rdi = r9;
    rsi = r8;
    var_50 = rdx;
    rbp = rcx;
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    rbx = *(u64*)((rbp + 0xADDR));
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    sub_14000217c(rcx);
    rdx = *(u64*)((rdi + 0xADDR));
    ecx = *(u32*)(rdx);
    rcx += *(u64*)((rax + 0xADDR));
    eax = 0;
    var_38 = al;
    var_30 = 0;
    var_28 = 0;
    var_20 = rcx;
    sub_140003b4c(rbp, &var_50, rsi, rdi, (rcx + *(u64*)((rax + 0xADDR))), 0, 0, 0);
    return;
}
