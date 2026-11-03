// L1 structure recovery (20 blocks)
void sub_140002418_l1(void) {
    __asm("cpuid");
    r8d = 0;
    r11d = 0;
    r10d = edx;
    r8d ^= 0xADDR;
    r10d ^= 0xADDR;
    r9d = ebx;
    esi = 0;
    eax = (r11 + 1);
    r10d |= r8d;
    __asm("cpuid");
    r9d ^= 0xADDR;
    var_0 = eax;
    r10d |= r9d;
    var_4 = ebx;
    edi = 0;
    var_8 = 0;
    var_c = edx;
    if (r10d != 0) {
        r8d = dword_ADDR;
    } else {
        qword_ADDR = -1;
        eax &= 0xADDR;
        qword_ADDR = 0xADDR;
        if (eax == 0xADDR) {
            dword_ADDR |= 1;
        } else if (eax != 0xADDR) {
            if (eax != 0xADDR) {
                eax += 0xADDR;
                if (eax <= 0xADDR) {
                    rcx = 0xADDR;
                }
            }
        }
    }
    eax = 7;
    r9d = (rax - 5);
    if (esi >= 7 /* signed */) {
        __asm("cpuid");
        var_0 = eax;
        r11d = ebx;
        var_4 = ebx;
        var_8 = 0;
        var_c = edx;
        if (((ebx >> 9) & 1) != 0) {
            r8d |= r9d;
            dword_ADDR = r8d;
        }
    }
    dword_ADDR = 1;
    dword_ADDR = r9d;
    if (((edi >> 0xADDR) & 1) != 0) {
        dword_ADDR = r9d;
        ebx = 6;
        dword_ADDR = 6;
        if (((edi >> 0xADDR) & 1) != 0 && ((edi >> 0xADDR) & 1) != 0) {
            ecx = 0;
            __asm("xgetbv");
            rdx <<= 0xADDR;
            rdx |= rax;
            var_20 = rdx;
            rax = var_20;
            al &= bl;
            if (al == bl) {
                eax = (dword_ADDR | 8);
                dword_ADDR = 3;
                dword_ADDR = eax;
                if ((r11b & 0xADDR) != 0) {
                    eax |= 0xADDR;
                    dword_ADDR = 5;
                    dword_ADDR = eax;
                    r11d &= 0xADDR;
                    if (r11d == 0xADDR) {
                        rax = var_20;
                        al &= 0xADDR;
                        if (al == 0xADDR) {
                            dword_ADDR |= 0xADDR;
                            dword_ADDR = ebx;
                        }
                    }
                }
            }
        }
    }
    eax = 0;
    return;
}
