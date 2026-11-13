// L1 structure recovery (4 blocks)
void _onexit_l1(void) {
    rbx = rcx;
    if (qword_ADDR != -1) {
        sub_1400085e0(off_140023b48, rbx);
    } else {
        sub_140008564(rcx);
    }
    edx = 0;
    rdx = ((eax == 0) ? rbx : rdx);
    rax = rdx;
    return;
}
