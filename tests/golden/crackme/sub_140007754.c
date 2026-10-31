// L1 structure recovery (6 blocks)
void sub_140007754_l1(void) {
    r9 = rdx;
    if (rcx >= 0xADDR) {
        eax = 0;
    } else {
        edx = 0;
        __asm("div r8");
        if (r9 >= -1) {
            eax = 0;
            return;
        }
        rcx <<= 3;
        r9 *= r8;
        rax = rcx;
        rax = ~(rax);
        if (rax <= r9) {
            eax = 0;
            return;
        }
        rcx += r9;
        sub_140008e60(rcx, 1);
        ecx = 0;
        rbx = rax;
        sub_140008ee0(0);
        rax = rbx;
    }
    return;
}
