// L1 structure recovery (6 blocks)
void sub_1400022d4_l1(void) {
    ebx = 0;
    while (1) {
        rdx = off_140023c30;
        r8d = 0;
        rcx = (rbx + (rbx * 4));
        rcx = (rdx + (rcx * 8));
        sub_1400025d0(rcx, 0xADDR, 0);
        if (eax == 0) break;
        dword_ADDR = (dword_ADDR + 1);
        ebx += 1;
        if (ebx < 1) continue;
    }
    sub_14000231c();
    al = 0;
    return;
}
