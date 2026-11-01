// L1 structure recovery (12 blocks)
void sub_1400118a0_l1(void) {
    ebx = ecx;
    sub_14000c520();
    ebp = 0;
    edi = eax;
    if (ebx == 0) {
        r8d = 0;
        SendMessageW(qword_ADDR, (r8 + 0xADDR), 0, off_14002b210);
        rsi = qword_ADDR;
        dword_ADDR = 1;
        if (rsi != 0) {
            GetLastError();
            ebx = eax;
            CoTaskMemFree(rsi);
            SetLastError(ebx);
        }
        qword_ADDR = rbp;
        sub_14000fe3c(rcx);
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        LocalReAlloc(qword_ADDR, 2, 2);
        if (rax == 0) {
            rax = qword_ADDR;
        } else {
            qword_ADDR = rax;
        }
        LocalLock(rax);
        if (rax != 0) {
            *(u16*)(rax) = bp;
        }
        LocalUnlock(qword_ADDR);
        SendMessageW(qword_ADDR, 0xADDR, qword_ADDR, 0);
        eax = dword_ADDR;
        dword_ADDR = eax;
        sub_14000f9f0();
        ecx = 0;
        sub_14000c980(0);
        if (edi != 0) {
            ebx = dword_ADDR;
            edi = dword_ADDR;
            sub_140010eec(qword_ADDR);
            sub_140022fe8(off_1400361c0, eax, edi, ebx);
            sub_14000d9f0();
            sub_140023648(eax);
        }
    }
    return;
}
