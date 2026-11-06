// L1 structure recovery (27 blocks)
void sub_1400052e0_l1(void) {
    *(u8*)(rdx) = 0;
    bpl = 0;
    rbx = qword_ADDR;
    r14 = r8;
    rsi = rdx;
    rdi = rcx;
    if (rbx != 0) {
        GetCurrentThreadId();
        r8d = eax;
        r9d = eax;
        rax = 0xADDR;
        __asm("mul r9");
        rdx >>= 3;
        rcx = (rdx + (rdx * 4));
        rcx += rcx;
        r8 -= rcx;
        rbx = *(u64*)((rbx + (r8 * 8)));
        while (rbx != 0) {
            if (*(u32*)(rbx) != r9d) {
                rbx = *(u64*)((rbx + 8));
            }
            break;
        }
        if (rbx != 0 && *(u64*)(rbx) != 0) {
            *(u8*)(rsi) = bpl;
            sub_140005974(rdi, *(u64*)(rbx), rsi, r14);
            if (al != 0) {
                *(u64*)((rdi + 0xADDR)) = rsi;
            }
            rbx = *(u64*)(rbx);
            do {
                al = *(u8*)((rbx + 0xADDR));
                *(u8*)((rbx + 0xADDR)) = 1;
                if (al == 0) {
                    rcx = *(u64*)((rbx + 8));
                    rdx = rdi;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)(rax);
                    sub_140028010(*(u64*)((rbx + 8)));
                    bpl |= al;
                    *(u8*)((rbx + 0xADDR)) = 0;
                }
                rbx = *(u64*)((rbx + 0xADDR));
            } while (rbx != 0);
        }
    }
    rax = qword_ADDR;
    if (rax != 0) {
        if (bpl != 0) {
            cl = 1;
        } else if ((*(u8*)((rdi + 4)) & 2) == 0) {
            ecx = 0;
        }
        rdx = rdi;
        sub_140028010(rcx);
    }
    GetCurrentThreadId();
    ecx = dword_ADDR;
    if (ecx != eax) {
        dword_ADDR += 1;
        ecx = ((dword_ADDR - ecx) + 1);
        if (ecx < 4 /* signed */) {
            dword_ADDR = eax;
            sub_140005a6c();
            if (rax != 0) {
                sub_1400081ac(rax, rdi);
            }
            dword_ADDR = 0;
        }
        dword_ADDR -= 1;
    }
    return;
}
