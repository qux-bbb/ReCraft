// L1 structure recovery (1 blocks)
void sub_14001f420_l1(void) {
    r14d = ecx;
    MonitorFromWindow(qword_ADDR, 2);
    esi = 0;
    var_30 = 0;
    var_2c = 0;
    GetDpiForMonitor(rax, 0, &var_30, &var_2c);
    ebx = (rsi + 0xADDR);
    MulDiv(ebx, var_30, (rsi + 0xADDR));
    r14d -= eax;
    MulDiv(ebx, var_30, (rsi + 0xADDR));
    edi = r14d;
    edi -= eax;
    MulDiv((rsi + 0xADDR), var_30, (rsi + 0xADDR));
    r8d = (rsi + 0xADDR);
    ebx = edi;
    ebx -= eax;
    MulDiv((r8 + 0xADDR), var_30, (rsi + 0xADDR));
    ecx = ebx;
    ecx -= eax;
    eax = esi;
    eax = ((ecx > 0 /* signed */) ? ecx : eax);
    var_28 = eax;
    eax = esi;
    eax = ((ebx > 0 /* signed */) ? ebx : eax);
    var_24 = eax;
    eax = esi;
    eax = ((edi > 0 /* signed */) ? edi : eax);
    var_20 = eax;
    esi = ((r14d > 0 /* signed */) ? r14d : esi);
    var_18 = -1;
    var_1c = esi;
    SendMessageW(qword_ADDR, 0xADDR, 5, &var_28);
    SendMessageW(qword_ADDR, 5, 0, 0);
    return;
}
