// L1 structure recovery (4 blocks)
void sub_140001c24_l1(void) {
    rbx = rcx;
    if (qword_ADDR != -1) {
        rdx = rbx;
        sub_14000294c(off_140034ce0);
    } else {
        sub_1400028f8();
    }
    edx = 0;
    rdx = ((eax == 0) ? rbx : rdx);
    rax = rdx;
    return;
}
