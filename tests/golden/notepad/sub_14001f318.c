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
    r8d = 3;
    eax = -(eax);
    ebx = 0xADDR;
    r9 = ((eax != 0) ? -1 : 0);
    r9 = ~(r9);
    r9 &= r8;
    SendMessageW(qword_ADDR, ebx, 3, (r9 & r8));
    eax = dword_ADDR;
    r8d = 4;
    eax = -(eax);
    r9 = ((eax != 0) ? -1 : 0);
    r9 = ~(r9);
    r9 &= r8;
    SendMessageW(qword_ADDR, ebx, 4, (r9 & r8));
    r8d = 0xADDR;
    SendMessageW(qword_ADDR, ebx, 0xADDR, r8d);
    r8d = 8;
    SendMessageW(qword_ADDR, ebx, 8, r8d);
    rdx = qword_ADDR;
    return sub_140023d8c(qword_ADDR);
}
