// L1 structure recovery (10 blocks)
void sub_140020314_l1(void) {
    var_48 = 0;
    var_40 = 0;
    OpenPrinterW(off_140035850, &var_48, 0);
    if (eax != 0) {
        rax = &var_40;
        r9d = 0;
        var_28 = rax;
        var_20 = 0;
        edi = (r9 + 1);
        GetPrinterDriverW(var_48, 0, edi, 0, 0, &var_40);
        LocalAlloc((rdi + 0xADDR), var_40);
        rcx = var_48;
        rbx = rax;
        if (rax != 0) {
            rax = &var_40;
            var_28 = rax;
            eax = var_40;
            var_20 = eax;
            GetPrinterDriverW(*(u64*)((rbp + 0xADDR)), 0, edi, rbx, var_40, &var_40);
            if (eax != 0) {
                dword_ADDR = (dword_ADDR | 0xADDR);
                sub_14000f5b8(off_140035550);
                dword_ADDR = (dword_ADDR & 0xADDR);
                CreateDCW(*(u64*)(rbx), off_140035850, 0, 0);
                rdi = rax;
                LocalFree(rbx);
                ClosePrinter(var_48);
                if (rdi != 0) {
                    rax = rdi;
                } else {
                    MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, (rdi + 0xADDR));
                    eax = 0;
                }
            } else {
                LocalFree(rbx);
                rcx = var_48;
                ClosePrinter(*(u64*)((rbp + 0xADDR)));
                eax = 0;
                return;
            }
        } else {
            ClosePrinter(*(u64*)((rbp + 0xADDR)));
            eax = 0;
            return;
        }
    } else {
        eax = 0;
        return;
    }
    return;
}
