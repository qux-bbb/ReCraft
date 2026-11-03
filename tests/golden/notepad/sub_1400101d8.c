// L1 structure recovery (38 blocks)
void sub_1400101d8_l1(void) {
    xmm0 = 0;
    r14d = r9d;
    msg = xmm0;
    rsi = rcx;
    var_50 = xmm0;
    var_60 = xmm0;
    GetCommandLineW();
    rbx = rax;
    CoCreateGuid(off_140036178);
    rax = qword_ADDR;
    r15d = 0;
    qword_ADDR = r15;
    xmm0 = *(__m128*)((rax - 0xADDR));
    var_70 = xmm0;
    if (qword_ADDR == 0) {
        xmm0 = 0;
        xmmword_ADDR = xmm0;
        EventRegister(&var_70, sub_140001260, off_1400341f8, off_140034218);
        if (eax == 0) {
            EventSetInformation(qword_ADDR, (rax + 2), qword_ADDR, (uint32)(*(u16*)(qword_ADDR)));
        }
        sub_140023784(rcx);
        r9d = 0;
        HeapSetInformation(0, (r9 + 1), 0, 0);
        CoInitializeEx(0, 2);
        if (eax >= 0 /* signed */) {
            eax = (uint32)(*(u16*)(rbx));
            edi = r15d;
            while (ax != 0) {
                if (ax == 0xADDR) {
                    if (edi == 0) break;
                    if (ax == 0xADDR) {
                        eax = r15d;
                        al = (uint8)((edi == 0));
                        edi = eax;
                    }
                    CharNextW(rbx);
                    rbx = rax;
                    eax = (uint32)(*(u16*)(rax));
                }
            }
            while (1) {
                if (*(u16*)(rbx) == 0xADDR) {
                    rbx += 2;
                    continue;
                }
                if (*(u16*)(rbx) != 9) break;
            }
            sub_140013154(rsi, rsi, rbx, r14d);
            if (eax != 0) {
                sub_14000d9f0();
                sub_1400236f8(eax);
                GetCurrentProcessId();
                ecx = 0xADDR;
                var_30 = r15d;
                var_28 = r15d;
                var_20 = eax;
                SetWinEventHook(0xADDR, ecx, 0, 0xADDR, eax, r15d, r15d);
                rsi = rax;
                loc_ADDR:
                while (1) {
                    GetMessageW(&msg, 0, 0, 0);
                    if (eax == 0) break;
                    if (msg.message == 0xADDR) {
                        PostMessageW(qword_ADDR, 0xADDR, 0, 0);
                    }
                    TranslateAcceleratorW(qword_ADDR, qword_ADDR, &msg);
                    if (eax != 0) continue;
                }
                ebx = dword_ADDR;
                edi = dword_ADDR;
                sub_140010eec(qword_ADDR);
                sub_140022fe8(off_1400361c0, eax, edi, ebx);
                rcx = qword_ADDR;
                if (rcx != 0) {
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010();
                    qword_ADDR = r15;
                }
                sub_14000cb7c();
                if (rsi != 0) {
                    UnhookWinEvent(rsi);
                }
            }
            rcx = qword_ADDR;
            if (rcx != 0) {
                sub_1400271b0(rcx);
                qword_ADDR = r15;
            }
            rcx = qword_ADDR;
            if (rcx != 0) {
                sub_14002708c(rcx);
                qword_ADDR = r15;
            }
            CoUninitialize();
        }
        rcx = qword_ADDR;
        qword_ADDR = r15;
        dword_ADDR = r15d;
        EventUnregister(qword_ADDR);
        eax = msg.wParam;
        return;
    }
    ecx = (r15 + 5);
    __asm("int 0xADDR");
    rcx = qword_ADDR;
    if (rcx != 0) {
        IsDialogMessageW(qword_ADDR, &msg);
        if (eax != 0) goto loc_ADDR;
    }
    rcx = qword_ADDR;
    if (rcx == 0) {
        TranslateAcceleratorW(qword_ADDR, qword_ADDR, &msg);
        if (eax != 0) goto loc_ADDR;
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        rcx = qword_ADDR;
        qword_ADDR = r15;
        dword_ADDR = r15d;
        EventUnregister(qword_ADDR);
        eax = msg.wParam;
        return;
    }
    IsDialogMessageW(qword_ADDR, &msg);
    if (eax != 0) goto loc_ADDR;
    TranslateAcceleratorW(qword_ADDR, qword_ADDR, &msg);
    if (eax != 0) goto loc_ADDR;
    TranslateMessage(&msg);
    DispatchMessageW(&msg);
}
