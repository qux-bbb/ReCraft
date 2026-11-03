// L1 structure recovery (14 blocks)
void sub_140021020_l1(void) {
    rsi = rcx;
    if (edx == 2) {
        ebx = 0xADDR;
        rdi = off_140035780;
        do {
            GetDlgItemTextW(rsi, ebx, rdi, 0xADDR);
            ebx += 1;
            rdi += 0xADDR;
        } while (ebx <= 0xADDR /* signed */);
        eax = 0;
    } else if (edx == 0xADDR) {
        if (r8d != 0xADDR) {
            eax = 0;
            return;
        }
        eax = (*(u32*)((r9 + 0xADDR)) + 4);
        if ((eax & 0xADDR) != 0) {
            eax = 0;
            return;
        }
        sub_14001fd94("https://go.microsoft.com/fwlink/p/?linkid=838060");
        eax = 1;
    } else if (edx != 0xADDR) {
        eax = 0;
        return;
    }
    ebx = 0xADDR;
    rbp = off_1400356e0;
    edi = 0xADDR;
    do {
        var_20 = 0;
        SendDlgItemMessageW(rsi, edi, 0xADDR, 0xADDR, 0);
        SetDlgItemTextW(rsi, edi, rbp);
        edi += 1;
        rbp += 0xADDR;
    } while (edi <= 0xADDR /* signed */);
    var_20 = 0xADDR;
    SendDlgItemMessageW(rsi, ebx, 0xADDR, 0, 0xADDR);
    return;
}
