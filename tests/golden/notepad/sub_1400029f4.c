// L1 structure recovery (1 blocks)
void sub_1400029f4_l1(void) {
    rbx = r9;
    rdi = r8;
    rsi = rdx;
    rbp = rcx;
    sub_1400016f4();
    var_28 = rbx;
    r9 = rdi;
    var_20 = 0;
    r8 = rsi;
    rdx = rbp;
    rcx = *(u64*)(rax);
    rcx |= 1;
    sub_1400028bc(rcx, rbp, rsi, rdi);
    eax = ((eax < 0 /* signed */) ? -1 : eax);
    return;
}
