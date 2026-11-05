// L1 structure recovery (1 blocks)
void sub_140002698_l1(void) {
    rbx = rcx;
    rax = rdx;
    rcx = "`'";
    *(u64*)(rbx) = rcx;
    rdx = (rbx + 8);
    *(__m128*)(rdx) = 0;
    sub_1400028a4((rax + 8), (rbx + 8));
    *(u64*)(rbx) = "`'";
    rax = rbx;
    return;
}
