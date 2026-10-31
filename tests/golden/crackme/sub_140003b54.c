// L1 structure recovery (1 blocks)
void sub_140003b54_l1(void) {
    rbx = rcx;
    rax = rdx;
    rcx = off_1400194a0;
    xmm0 = 0;
    *(u64*)(rbx) = rcx;
    rdx = (rbx + 8);
    *(__m128*)(rdx) = xmm0;
    sub_140004310((rax + 8), (rbx + 8));
    *(u64*)(rbx) = off_1400194d0;
    rax = rbx;
    return;
}
