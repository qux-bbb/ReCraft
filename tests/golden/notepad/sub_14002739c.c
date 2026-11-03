// L1 structure recovery (3 blocks)
void sub_14002739c_l1(void) {
    r10 = *(u64*)((r9 + 0xADDR));
    rsi = rdx;
    r14 = r8;
    rbp = rcx;
    rdi = r9;
    ebx = *(u32*)(r10);
    rbx <<= 4;
    rbx += r10;
    sub_140027338(rsi, r9, (rbx + 4));
    al &= 0xADDR;
    al = -(al);
    edx = (((al != 0) ? 1 : 0) + 1);
    if ((*(u32*)((rbx + 4)) & edx) != 0) {
        r9 = rdi;
        r8 = r14;
        rdx = rsi;
        rcx = rbp;
        sub_14000288c(rbp, rsi, r14, rdi);
    }
    return;
}
