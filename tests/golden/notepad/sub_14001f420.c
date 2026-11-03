// L1 structure recovery (1 blocks)
void sub_14001f420_l1(void) {
    r14d = ecx;
    MonitorFromWindow(qword_ADDR, 2);
    esi = 0;
    var_20 = 0;
    var_24 = 0;
    GetDpiForMonitor(rax, 0, &var_20, &var_24);
    ebx = (rsi + 0xADDR);
    MulDiv(ebx, var_20, (rsi + 0xADDR));
    r14d -= eax;
    MulDiv(ebx, var_20, (rsi + 0xADDR));
    edi = r14d;
    edi -= eax;
    MulDiv((rsi + 0xADDR), var_20, (rsi + 0xADDR));
    r8d = (rsi + 0xADDR);
    ebx = edi;
    ebx -= eax;
    MulDiv((r8 + 0xADDR), var_20, (rsi + 0xADDR));
    ecx = ebx;
    ecx -= eax;
    eax = esi;
    eax = ((ecx > 0 /* signed */) ? ecx : eax);
    var_28 = eax;
    eax = esi;
    eax = ((ebx > 0 /* signed */) ? ebx : eax);
    var_2c = eax;
    ebx = 5;
    eax = esi;
    eax = ((edi > 0 /* signed */) ? edi : eax);
    var_30 = eax;
    esi = ((r14d > 0 /* signed */) ? r14d : esi);
    var_38 = -1;
    var_34 = esi;
    SendMessageW(qword_ADDR, 0xADDR, ebx, &var_28);
    SendMessageW(qword_ADDR, ebx, 0, 0);
    r11 = &var_50;
    return;
}
