// L1 structure recovery (1 blocks)
void sub_1400042d0_l1(void) {
    var_40 = r8d;
    rdx = *(u64*)(rdx);
    rax = rcx;
    sub_1400020f0(rcx, rdx, r8);
    (*rax)();
    sub_140002120();
    rdx = *(u64*)(rdx);
    sub_1400020f0(rax, rdx, 2);
    return;
}
