// L1 structure recovery (36 blocks)
void sub_140003624_l1(void) {
    rdi = var_1538;
    r14 = r8;
    esi = edx;
    r15 = rcx;
    edx = 0;
    sub_1400029dc(&var_20, 0, 0xADDR);
    rdx = var_1540;
    r12d = 0;
    var_4c0 = r12w;
    var_c0 = r12b;
    ecx = *(u32*)(rdx);
    eax = *(u32*)((rdx + 4));
    var_28 = ecx;
    var_2c = eax;
    sub_140006e9c(*(u32*)(rdx));
    ebx = eax;
    eax = (r12 + 8);
    var_20 = 1;
    ecx = r12d;
    ecx = ((*(u32*)((rdx + 8)) == 1) ? eax : ecx);
    var_24 = ecx;
    ecx = (rax - 7);
    __asm("lock xadd [0xADDR], ecx");
    ecx += 1;
    var_38 = r12;
    var_30 = ecx;
    GetCurrentThreadId();
    xmm0 = 0;
    var_58 = r14;
    var_40 = eax;
    xmm1 = 0;
    eax = 0;
    var_60 = esi;
    var_98 = rax;
    var_80 = rax;
    rax = qword_ADDR;
    var_64 = ebx;
    var_48 = r12;
    var_50 = r12;
    var_a8 = rdi;
    var_b0 = r15;
    var_68 = r12;
    var_88 = xmm0;
    var_70 = xmm1;
    if (rax == 0) {
        var_a0 = r12;
    } else {
        sub_140028010(rcx, rdx, r8);
        var_a0 = rax;
    }
    rax = qword_ADDR;
    if (rax != 0) {
        sub_140028010(&var_20);
    }
    rax = qword_ADDR;
    if (rax != 0) {
        sub_140028010(&var_20, &var_c0, 0xADDR);
    }
    rax = qword_ADDR;
    if (rax != 0) {
        sub_140028010(&var_20);
    }
    rax = qword_ADDR;
    if (rax != 0 && (var_24 & 2) == 0) {
        sub_140028010(&var_20);
    }
    if (var_28 >= r12d /* signed */) {
        sub_1400091d4();
    }
    if (byte_ADDR != r12b) {
        if ((var_24 & 2) == 0) {
            rax = qword_ADDR;
            ebx = 0xADDR;
            if (rax != 0 && byte_ADDR == r12b) {
                r8d = ebx;
                sub_140028010(&var_20, &var_4c0);
            }
            if (var_4c0 == r12w) {
                sub_1400054a4(&var_4c0, rbx, &var_20);
            }
            OutputDebugStringW(&var_4c0);
        } else {
            rax = qword_ADDR;
            if (rax != 0 && byte_ADDR == r12b) {
                r8d = 0;
                edx = 0;
                sub_140028010(&var_20);
            }
        }
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            IsDebuggerPresent();
        } else {
            sub_140028010(rcx, rdx, r8);
        }
        __asm("if (je short 0xADDR /* unmodelled flags */) goto 0xADDR");
    }
    if ((var_24 & 4) != 0) {
        rax = qword_ADDR;
        if (rax != 0) {
            sub_140028010(rcx, rdx, r8);
        }
    }
    if ((var_24 & 1) != 0) {
        sub_140008fec(&var_20);
    }
    return;
}
