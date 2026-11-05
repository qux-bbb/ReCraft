// L1 structure recovery (35 blocks)
void sub_140001a34_l1(void) {
    ecx = 0;
    __asm("cpuid");
    ecx ^= 0xADDR;
    edx ^= 0xADDR;
    edx |= ecx;
    ebp = 0;
    eax = 1;
    ebx ^= 0xADDR;
    edx |= ebx;
    ecx = (rax - 1);
    __asm("cpuid");
    edi = ecx;
    if (edx != 0) {
        r8d = dword_ADDR;
    } else {
        eax &= 0xADDR;
        qword_ADDR = 0xADDR;
        qword_ADDR = -1;
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
    r9d = 0;
    esi = 0;
    r10d = 0;
    r11d = 0;
    if (ebp >= 7 /* signed */) {
        eax = (r9 + 7);
        ecx = 0;
        __asm("cpuid");
        esi = edx;
        r9d = ebx;
        if (((ebx >> 9) & 1) != 0) {
            r8d |= 2;
            dword_ADDR = r8d;
        }
        if (eax >= 1 /* signed */) {
            eax = 7;
            ecx = (rax - 6);
            __asm("cpuid");
            r10d = edx;
        }
        if (ebp >= 0xADDR /* signed */) {
            ecx = 0;
            __asm("cpuid");
            r11d = ebx;
        }
    }
    rax = (qword_ADDR & -0xADDR);
    dword_ADDR = 1;
    dword_ADDR = 2;
    qword_ADDR = rax;
    if (((edi >> 0xADDR) & 1) != 0) {
        rax &= -0xADDR;
        dword_ADDR = 2;
        qword_ADDR = rax;
        dword_ADDR = 6;
    }
    if (((edi >> 0xADDR) & 1) != 0) {
        ecx = 0;
        __asm("xgetbv");
        rdx <<= 0xADDR;
        rdx |= rax;
        var_20 = rdx;
        if (((edi >> 0xADDR) & 1) != 0) {
            rax = var_20;
            al &= 6;
            if (al == 6) {
                eax = dword_ADDR;
                dl = 0xADDR;
                eax |= 8;
                dword_ADDR = 3;
                dword_ADDR = eax;
                if ((r9b & 0xADDR) == 0) {
                    rax = qword_ADDR;
                } else {
                    eax |= 0xADDR;
                    dword_ADDR = 5;
                    dword_ADDR = eax;
                    rax = qword_ADDR;
                    r9d &= 0xADDR;
                    rax &= -0xADDR;
                    qword_ADDR = rax;
                    if (r9d == 0xADDR) {
                        rax = var_20;
                        al &= 0xADDR;
                        if (al == 0xADDR) {
                            rax = qword_ADDR;
                            dword_ADDR |= 0xADDR;
                            rax &= -0xADDR;
                            dword_ADDR = 6;
                            qword_ADDR = rax;
                        }
                    }
                }
                if (((esi >> 0xADDR) & 1) != 0) {
                    rax &= 0xADDR;
                    qword_ADDR = rax;
                }
                if (((r10d >> 0xADDR) & 1) != 0) {
                    rax = var_20;
                    al &= dl;
                    if (al == dl) {
                        ecx = r11d;
                        eax = r11d;
                        rcx >>= 0xADDR;
                        eax &= 0xADDR;
                        ecx &= 6;
                        dword_ADDR = eax;
                        rcx |= 0xADDR;
                        rcx = ~(rcx);
                        rcx &= qword_ADDR;
                        qword_ADDR = rcx;
                        if (al > 1) {
                            rcx &= -0xADDR;
                            qword_ADDR = rcx;
                        }
                    }
                }
            }
        }
        if (((r10d >> 0xADDR) & 1) != 0) {
            rax = var_20;
            if (((rax >> 0xADDR) & 1) != 0) {
                qword_ADDR &= -0xADDR;
            }
        }
    }
    eax = 0;
    return;
}
