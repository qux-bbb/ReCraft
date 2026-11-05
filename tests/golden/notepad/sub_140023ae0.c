// L1 structure recovery (4 blocks)
void sub_140023ae0_l1(void) {
    ebx = 0;
    r9d = edx;
    r8 = rcx;
    var_28 = 0;
    MultiByteToWideChar(0xADDR, (rbx + 8), rcx, edx, 0, 0);
    if (eax != 0) {
        ebx = 1;
    } else {
        GetLastError();
        if (eax != 0xADDR) {
            eax = 1;
            return;
        }
    }
    eax = 1;
    return;
}
