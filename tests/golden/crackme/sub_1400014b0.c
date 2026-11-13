// L1 structure recovery (12 blocks)
void sub_1400014b0_l1(void) {
    ebx = ecx;
    if (byte_ADDR != 0) {
        al = 1;
        return;
    }
    if (ecx > 1) {
        __scrt_fastfail(5);
    }
    sub_140001ccc();
    if ((eax == 0) || (ebx != 0)) {
        xmm0 = xmmword_ADDR;
        xmmword_ADDR = xmm0;
        qword_ADDR = -1;
        xmmword_ADDR = xmm0;
        qword_ADDR = -1;
        byte_ADDR = 1;
        al = 1;
        return;
    }
    sub_1400085b8(off_140023b48);
    if (eax != 0) {
        al = 0;
        return;
    }
    sub_1400085b8(off_140023b60);
    if (eax == 0) {
        byte_ADDR = 1;
        al = 1;
        return;
    }
    al = 0;
    return;
}
