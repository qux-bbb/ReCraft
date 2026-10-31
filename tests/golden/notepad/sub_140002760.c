// L1 structure recovery (3 blocks)
void sub_140002760_l1(void) {
    rax = "`'";
    rdi = rcx;
    *(u64*)(rcx) = rax;
    ebx = edx;
    rcx += 8;
    sub_1400028b0(rcx);
    if ((bl & 1) != 0) {
        rcx = rdi;
        sub_1400025e0(rdi);
    }
    rax = rdi;
    return;
}
