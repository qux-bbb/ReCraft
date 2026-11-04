// L1 structure recovery (34 blocks)
void sub_1400033ac_l1(void) {
    rsi = var_1528;
    r15 = r8;
    r14d = edx;
    r12 = rcx;
    edx = 0;
    sub_1400029dc(&var_20, 0, 0xADDR);
    rbx = var_1530;
    r13d = 0;
    var_4c0 = r13w;
    var_c0 = r13b;
    ecx = *(u32*)(rbx);
    eax = *(u32*)((rbx + 4));
    var_28 = ecx;
    var_2c = eax;
    sub_140006d20(*(u32*)(rbx));
    edi = eax;
    eax = (r13 + 8);
    var_20 = 3;
    ecx = r13d;
    ecx = ((*(u32*)((rbx + 8)) == 1) ? eax : ecx);
    var_24 = ecx;
    ecx = (rax - 7);
    dword_ADDR += ecx;
    ecx = ((dword_ADDR - ecx) + 1);
    var_38 = r13;
    var_30 = ecx;
    GetCurrentThreadId();
    xmm0 = 0;
    var_58 = r15;
    var_40 = eax;
    xmm1 = 0;
    var_60 = r14d;
    var_98 = 0;
    var_80 = 0;
    rax = qword_ADDR;
    var_64 = edi;
    var_48 = r13;
    var_50 = r13;
    var_a8 = rsi;
    var_b0 = r12;
    var_68 = r13;
    var_88 = xmm0;
    var_70 = xmm1;
    if (rax == 0) {
        var_a0 = r13;
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
    if (var_28 >= r13d /* signed */) {
        ecx = 0xADDR;
        var_28 = 0xADDR;
        sub_140005cc0(0xADDR);
        var_2c = eax;
    }
    if (byte_ADDR != r13b) {
        if ((var_24 & 2) == 0) {
            rax = qword_ADDR;
            ebx = 0xADDR;
            if (rax != 0 && byte_ADDR == r13b) {
                r8d = ebx;
                sub_140028010(&var_20, &var_4c0);
            }
            if (var_4c0 == r13w) {
                sub_1400054a4(&var_4c0, rbx, &var_20);
            }
            OutputDebugStringW(&var_4c0);
        } else {
            rax = qword_ADDR;
            if (rax != 0 && byte_ADDR == r13b) {
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
    sub_140008fec(&var_20);
}
