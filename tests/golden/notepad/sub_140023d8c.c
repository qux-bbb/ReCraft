// L1 structure recovery (5 blocks)
void sub_140023d8c_l1(void) {
    rdi = rcx;
    eax = 0;
    rcx = qword_ADDR;
    rbx = rdx;
    if (rcx == 0) {
        r8d = (rcx + 3);
        var_20 = off_140036660;
        CoCreateInstance(off_14002bb90, 0, (rcx + 3), off_14002bb70, off_140036660);
        rcx = qword_ADDR;
    }
    if (eax >= 0 /* signed */) {
        rax = *(u64*)(rcx);
        rdx = &var_40;
        xmm0 = xmmword_ADDR;
        var_28 = rbx;
        r9d = 0;
        var_20 = rdx;
        rax = *(u64*)((rax + 0xADDR));
        var_40 = xmm0;
        sub_140028010(rcx, rdi, 0xADDR);
    }
    return;
}
