// L1 structure recovery (10 blocks)
void sub_1400083f8_l1(void) {
    var_78 = r9;
    rax = (rdx - 1);
    ebx = 0;
    r14 = rcx;
    if (rax <= 0xADDR) {
        rsi = (rdx - 1);
        edi = 0;
        sub_1400029f4(rcx, rsi, r8, &var_78);
        if (eax < 0 /* signed */) {
            edi = 0xADDR;
            *(u16*)((r14 + (rsi * 2))) = 0xADDR;
        } else {
            rax = (int64)(eax);
            if (rax > rsi) {
                *(u16*)((r14 + (rsi * 2))) = 0xADDR;
                eax = 0xADDR;
                return;
            } else if (rax == rsi) {
                *(u16*)((r14 + (rsi * 2))) = bx;
                eax = 0xADDR;
                return;
            }
        }
    } else {
        if (rdx != 0) {
            *(u16*)(rcx) = bx;
        }
    }
    eax = 0xADDR;
    return;
}
