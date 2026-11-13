// L1 structure recovery (1 blocks)
void sub_140003b54_l1(void) {
    rbx = rcx;
    rax = rdx;
    *(u64*)(rbx) = off_1400194a0;
    rdx = (rbx + 8);
    *(__m128*)(rdx) = 0;
    __std_exception_copy((rax + 8), (rbx + 8));
    *(u64*)(rbx) = off_1400194d0;
    rax = rbx;
    return;
}
