// L1 structure recovery (12 blocks)
void sub_140020260_l1(void) {
    xmm0 = 0;
    msg = xmm0;
    var_40 = xmm0;
    var_50 = xmm0;
    if (dword_ADDR != 0) {
        ebx = 0;
    } else {
        ebx = 1;
        while (1) {
            var_20 = ebx;
            PeekMessageW(&msg, 0, 0, 0, ebx);
            if (eax == 0) break;
            if (qword_ADDR == 0) {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                IsDialogMessageW(qword_ADDR, &msg);
            }
            eax = dword_ADDR;
            if (eax == 0) continue;
        }
        eax = dword_ADDR;
        if (eax != 0) {
            ebx = 0;
            eax = 0;
            return;
        }
    }
    eax = ebx;
    return;
}
