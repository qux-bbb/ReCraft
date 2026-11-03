// L1 structure recovery (12 blocks)
void sub_140001a94_l1(void) {
    ebx = ecx;
    if (byte_ADDR != 0) {
        al = 1;
        return;
    }
    if (ecx > 1) {
        sub_140002100(5);
    }
    sub_1400025cc();
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
    sub_14000291c(off_140034ce0);
    if (eax != 0) {
        al = 0;
        return;
    }
    sub_14000291c(off_140034cf8);
    if (eax == 0) {
        byte_ADDR = 1;
        al = 1;
        return;
    }
    al = 0;
    return;
}
