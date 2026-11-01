// L1 structure recovery (5 blocks)
void sub_14001eaf4_l1(void) {
    rdi = (int64)(var_60);
    ebp = 0;
    var_40 = ebp;
    rbx = r9;
    *(u16*)(r9) = bp;
    rsi = r8;
    eax = (rdi + rdi);
    var_60 = eax;
    if (rcx == 0) {
        r8 = rdi;
        rdx = rsi;
        r8 += r8;
        rcx = rbx;
        sub_1400274ac(rbx, rsi, r8);
    } else {
        *(u64*)((r11 - 0xADDR)) = (r11 + 0xADDR);
        *(u64*)((r11 - 0xADDR)) = rbx;
        RegQueryValueExW(rcx, rdx, 0, (r11 + 8));
        if (eax != 0) {
            r8 = rdi;
            rdx = rsi;
            r8 += r8;
            rcx = rbx;
            sub_1400274ac(rbx, rsi, r8);
            return;
        } else if (var_40 != 1) {
            r8 = rdi;
            rdx = rsi;
            r8 += r8;
            rcx = rbx;
            sub_1400274ac(rbx, rsi, r8);
            return;
        }
    }
    return;
}
