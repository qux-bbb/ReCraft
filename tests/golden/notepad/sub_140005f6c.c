// L1 structure recovery (52 blocks)
void sub_140005f6c_l1(void) {
    var_88 = edx;
    var_80 = rcx;
    rax = var_e0;
    r13 = r8;
    r15 = var_b8;
    r8d = 0;
    rbx = var_f0;
    r10 = rcx;
    ebp = var_b0;
    r12 = r9;
    r14 = var_d0;
    esi = 0;
    ecx = ebp;
    *(u16*)(r14) = r8w;
    *(u8*)(rax) = r8b;
    edi = *(u32*)(r15);
    *(u32*)((rbx + 8)) = edi;
    eax = *(u32*)((r15 + 4));
    *(u32*)((rbx + 0xADDR)) = eax;
    if (ebp == 0) {
        sub_140006d00(edi);
        esi = eax;
    } else {
        ecx -= 1;
        if (ecx == 0) {
            sub_140006e9c(edi);
            esi = eax;
            return;
        }
        ecx -= 1;
        if (ecx == 0) {
            if (edi >= 0 /* signed */) {
                rax = var_a8;
                edi = 0xADDR;
                var_30 = 0xADDR;
                var_28 = rax;
                rax = var_a0;
                var_20 = rax;
                sub_1400032f0(r10, rdx, r13, r9, var_a0, var_a8, 0xADDR);
                *(u32*)((rbx + 8)) = 0xADDR;
                sub_140005cc0(edi);
                *(u32*)((rbx + 0xADDR)) = eax;
            }
            sub_140006e7c(edi);
            esi = eax;
            return;
        } else if (ecx == 1) {
            sub_140006d20(edi);
            esi = eax;
            return;
        }
    }
    edi = 0;
    *(u32*)(rbx) = ebp;
    *(u32*)((rbx + 4)) = 0;
    if (*(u32*)((r15 + 8)) == 1) {
        *(u32*)((rbx + 4)) = 8;
    }
    eax = 1;
    __asm("lock xadd [0xADDR], eax");
    eax += 1;
    *(u32*)((rbx + 0xADDR)) = eax;
    rax = var_c0;
    if (rax == 0) {
        rax = rdi;
    }
    *(u64*)((rbx + 0xADDR)) = rax;
    GetCurrentThreadId();
    *(u64*)((rbx + 0xADDR)) = r13;
    xmm0 = 0;
    *(u32*)((rbx + 0xADDR)) = eax;
    *(u32*)((rbx + 0xADDR)) = var_88;
    *(u64*)((rbx + 0xADDR)) = var_a0;
    *(u64*)((rbx + 0xADDR)) = var_a8;
    rax = var_80;
    *(u64*)((rbx + 0xADDR)) = rax;
    *(u32*)((rbx + 0xADDR)) = esi;
    *(u64*)((rbx + 0xADDR)) = r12;
    *(u64*)((rbx + 0xADDR)) = rdi;
    *(__m128*)((rbx + 0xADDR)) = xmm0;
    *(u64*)((rbx + 0xADDR)) = 0;
    *(__m128*)((rbx + 0xADDR)) = xmm0;
    *(u64*)((rbx + 0xADDR)) = 0;
    rax = qword_ADDR;
    if (rax == 0) {
        rax = rdi;
    } else {
        sub_140028010(rcx, rdx, r8);
    }
    *(u64*)((rbx + 0xADDR)) = rax;
    rax = qword_ADDR;
    if (rax != 0) {
        rcx = rbx;
        sub_140028010(rbx);
    }
    rax = qword_ADDR;
    if (rax != 0) {
        sub_140028010(rbx, var_e0, 0xADDR);
    }
    rax = qword_ADDR;
    if (rax != 0) {
        rcx = rbx;
        sub_140028010(rbx);
    }
    rax = qword_ADDR;
    if (rax != 0 && (*(u8*)((rbx + 4)) & 2) == 0) {
        rcx = rbx;
        sub_140028010(rbx);
    }
    if (*(u32*)((rbx + 8)) < edi /* signed */) {
        if (byte_ADDR != dil) {
            if ((*(u8*)((rbx + 4)) & 2) == 0) {
                rax = qword_ADDR;
                esi = 0xADDR;
                if (rax != 0 && byte_ADDR == dil) {
                    r8d = esi;
                    rdx = r14;
                    rcx = rbx;
                    sub_140028010(rbx, r14, esi);
                }
                if (*(u16*)(r14) == di) {
                    sub_1400054a4(r14, rsi, rbx);
                }
                OutputDebugStringW(r14);
            } else {
                rax = qword_ADDR;
                if (rax != 0 && byte_ADDR == dil) {
                    r8d = 0;
                    edx = 0;
                    rcx = rbx;
                    sub_140028010(rbx, 0, 0);
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
        if ((*(u8*)((rbx + 4)) & 4) != 0) {
            rax = qword_ADDR;
            if (rax != 0) {
                sub_140028010(rcx, rdx, r8);
            }
        }
        return;
    }
    if (ebp != 3) {
        sub_1400091d4();
    }
    *(u32*)((rbx + 8)) = 0xADDR;
    sub_140005cc0(0xADDR);
    *(u32*)((rbx + 0xADDR)) = eax;
}
