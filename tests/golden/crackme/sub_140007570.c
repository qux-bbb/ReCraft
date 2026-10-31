// L1 structure recovery (1 blocks)
void sub_140007570_l1(void) {
    rdx = qword_ADDR;
    r8 = rcx;
    eax = edx;
    ecx = 0xADDR;
    eax &= 0xADDR;
    ecx -= eax;
    r8 = ror(r8, cl);
    r8 ^= rdx;
    qword_ADDR = r8;
    return;
}
