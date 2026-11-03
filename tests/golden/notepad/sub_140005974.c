// L1 structure recovery (14 blocks)
void sub_140005974_l1(void) {
    al = 0;
    *(u8*)(r8) = 0;
    rbp = r9;
    rdi = r8;
    rsi = rdx;
    rbx = rcx;
    if (rdx != 0) {
        rdx = *(u64*)((rdx + 0xADDR));
        sub_140005974();
        rdx = *(u64*)((rsi + 0xADDR));
        if (rdx != 0) {
            if (*(u32*)(rdx) == 0) {
                eax = 1;
                __asm("lock xadd [0xADDR], eax");
                eax += 1;
                *(u32*)(rdx) = eax;
            }
            if (*(u32*)((rbx + 0xADDR)) == 0) {
                xmm0 = *(__m128*)(rdx);
                *(__m128*)((rbx + 0xADDR)) = xmm0;
                xmm1 = *(f64*)((rdx + 0xADDR));
                *(f64*)((rbx + 0xADDR)) = xmm1;
            }
            xmm0 = *(__m128*)(rdx);
            rcx = (rdi + rbp);
            *(__m128*)((rbx + 0xADDR)) = xmm0;
            xmm1 = *(f64*)((rdx + 0xADDR));
            *(f64*)((rbx + 0xADDR)) = xmm1;
            rax = -1;
            do {
                rax += 1;
            } while (*(u8*)((rdi + rax)) != 0);
            rdi += rax;
            rax = rcx;
            rax -= rdi;
            if (rax > 2 /* signed */) {
                *(u8*)(rdi) = 0xADDR;
                rdi += 1;
                r8 = *(u64*)((rdx + 8));
                do {
                    rbx += 1;
                } while (*(u8*)((r8 + rbx)) != 0);
                rcx -= rdi;
                rbx += 1;
                rdx = rcx;
                rbx = ((rbx >= rcx) ? rcx : rbx);
                r9 = rbx;
                sub_1400096fc(rdi, rcx, r8);
                *(u8*)(((rbx + rdi) - 1)) = 0;
            }
            al = 1;
        }
    }
    return;
}
