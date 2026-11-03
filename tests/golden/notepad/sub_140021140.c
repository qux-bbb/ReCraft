// L1 structure recovery (13 blocks)
void sub_140021140_l1(void) {
    rsi = rcx;
    rbx = (int64)(edx);
    r14d = 0;
    var_60 = r14;
    rcx = (((rbx + (rbx * 4)) << 4) + off_1400356e0);
    if (*(u16*)(rcx) != 0xADDR) {
        sub_140021474(rcx);
        if (ebx != 0) {
            edi = dword_ADDR;
            edi -= dword_ADDR;
            edi -= dword_ADDR;
        } else {
            edi = dword_ADDR;
        }
        r15d = 0xADDR;
        if (word_ADDR != r14w) {
            wcsnlen(off_140035a90, r15d);
            var_20 = eax;
            TextOutW(rsi, dword_ADDR, edi, off_140035a90, eax);
        }
        if (word_ADDR != r14w) {
            rbp = off_140035c90;
            wcsnlen(rbp, r15);
            GetTextExtentPoint32W(rsi, rbp, eax, &var_60);
            eax = dword_ADDR;
            eax -= dword_ADDR;
            eax += dword_ADDR;
            edx = ((eax < 0 /* signed */) ? -1 : 0);
            eax -= edx;
            eax >>= 1;
            ebx = eax;
            eax = var_60;
            edx = ((eax < 0 /* signed */) ? -1 : 0);
            eax -= edx;
            eax >>= 1;
            ebx -= eax;
            wcsnlen(rbp, r15);
            var_20 = eax;
            TextOutW(rsi, ebx, edi, rbp, eax);
        }
        if (word_ADDR != r14w) {
            rbp = off_140035e90;
            wcsnlen(rbp, r15);
            GetTextExtentPoint32W(rsi, rbp, eax, &var_60);
            ebx = dword_ADDR;
            ebx -= dword_ADDR;
            ebx -= var_60;
            wcsnlen(rbp, r15);
            var_20 = eax;
            TextOutW(rsi, ebx, edi, rbp, eax);
        }
        eax = 1;
    } else {
        eax = 0;
    }
    return;
}
