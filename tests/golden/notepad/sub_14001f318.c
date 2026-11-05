// L1 structure recovery (3 blocks)
void sub_14001f318_l1(void) {
    rbx = rcx;
    rcx = qword_ADDR;
    if (rcx != 0) {
        sub_140023d18(rcx);
        DestroyWindow(qword_ADDR);
    }
    qword_ADDR = rbx;
    SendMessageW(rbx, 0xADDR, 0, 0);
    eax = dword_ADDR;
    eax = -(eax);
    ebx = 0xADDR;
    r9 = ((eax != 0) ? -1 : 0);
    r9 = (~(r9) & 3);
    SendMessageW(qword_ADDR, 0xADDR, 3, (r9 & 3));
    eax = dword_ADDR;
    eax = -(eax);
    r9 = ((eax != 0) ? -1 : 0);
    r9 = (~(r9) & 4);
    SendMessageW(qword_ADDR, 0xADDR, 4, (r9 & 4));
    SendMessageW(qword_ADDR, 0xADDR, 0xADDR, 0xADDR);
    SendMessageW(qword_ADDR, ebx, 8, 8);
    rdx = qword_ADDR;
    return sub_140023d8c(qword_ADDR);
}
