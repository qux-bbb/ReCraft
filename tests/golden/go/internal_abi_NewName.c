// L1 structure recovery (35 blocks)
void internal/abi.NewName_l1(void) {
    while (1) {
        if (&var_80 <= *(u64*)((r14 + 0xADDR))) {
            var_28 = sil;
            var_29 = r8b;
            runtime.morestack_noctxt();
            esi = (uint32)(var_28);
            r8d = (uint32)(var_29);
            continue;
        }
        var_98 = rax;
        var_a8 = rcx;
        if (rbx >= 0xADDR /* signed */) {
            rdi = rax;
            esi = 0xADDR;
            r9d = 3;
            eax = 0;
            runtime.concatstring3(0xADDR, rdx, off_1400db041);
            runtime.convTstring();
            rbx = rax;
            rax = off_1400bd6e0;
            runtime.gopanic();
        }
        if (rdi >= 0xADDR /* signed */) {
            eax = 0;
            rdi = rcx;
            esi = 0xADDR;
            r9d = 3;
            runtime.concatstring3(0xADDR, rdx, off_1400db041);
            runtime.convTstring();
            rbx = rax;
            rax = off_1400bd6e0;
            runtime.gopanic();
        }
        var_b8 = sil;
        var_b9 = r8b;
        var_a0 = rbx;
        var_98 = rax;
        var_b0 = rdi;
        var_a8 = rcx;
        var_45 = 0;
        var_4d = 0;
        var_3b = 0;
        var_43 = 0;
        rax = &var_45;
        ecx = 0xADDR;
        rbx = rcx;
        internal/abi.writeVarint(0xADDR, rdx, r8);
        var_58 = rax;
        rax = &var_3b;
        ebx = 0xADDR;
        rdi = var_b0;
        internal/abi.writeVarint(rbx, rdx, r8);
        rdx = var_a0;
        rsi = var_58;
        r8 = ((rsi + rdx) + 1);
        r9 = (rdx + rsi);
        r10 = var_b0;
        if (r10 == 0) {
            r11d = (uint32)(var_b8);
            rdi = r8;
        } else {
            rdi = (r10 + rax);
            rdi = ((r9 + rdi) + 1);
            r11d = (uint32)(var_b8);
            r11d |= 2;
        }
        var_50 = rax;
        var_60 = r8;
        var_70 = r9;
        r12d = (uint32)(var_b9);
        if (r12b != 0) {
            r11d |= 8;
        }
        var_4f = r11b;
        var_68 = rdi;
        rax = off_1400bd760;
        rbx = rdi;
        runtime.makeslice();
        rcx = var_68;
        __asm("if (jbe 0xADDR /* unmodelled flags */) goto 0xADDR");
        edx = (uint32)(var_4f);
        *(u8*)(rax) = dl;
        rsi = (rcx - 1);
        rdx = (-(rdx) >> 0xADDR);
        edx &= 1;
        rdx += rax;
        rdi = var_58;
        if (rdi > 0xADDR) {
            runtime.panicBounds(rcx, rdx, r8, r9);
            runtime.panicBounds(rcx, rdx, r8, r9);
        }
        var_80 = rax;
        rsi = ((rdi < rsi /* signed */) ? rdi : rsi);
        rbx = &var_45;
        if (rbx != rdx) {
            runtime.memmove(rsi);
            rax = var_80;
            rcx = var_68;
            rdi = var_58;
        }
        rdx = (rdi + 1);
        if (rcx < rdx) {
            runtime.panicBounds(rcx, rdx, r8, r9);
        }
        rsi = rcx;
        rsi -= rdi;
        rsi -= 1;
        rdi = rsi;
        rsi = (-(rsi) >> 0xADDR);
        rdx &= rsi;
        rdx += rax;
        rsi = var_a0;
        rdi = ((rsi < rdi /* signed */) ? rsi : rdi);
        rbx = var_98;
        if (rbx != rdx) {
            runtime.memmove(rdi);
            rdx = var_b0;
            rax = var_80;
            rcx = var_68;
        } else {
            rdx = var_b0;
        }
        __asm("if (je 0xADDR /* unmodelled flags */) goto 0xADDR");
        rsi = var_60;
        if (rcx < rsi) {
            runtime.panicBounds(rcx, rdx, r8, r9);
        } else {
            rdi = var_70;
            rcx -= rdi;
            r8 = (rcx - 1);
            rdi = (-(rdi) >> 0xADDR);
            rsi &= rdi;
            rsi += rax;
            rdi = var_50;
            if (rdi > 0xADDR) {
                runtime.panicBounds(rcx, rdx, r8, r9);
            } else {
                r9 = r8;
                r8 = ((rdi < r8 /* signed */) ? rdi : r8);
                rbx = &var_3b;
                if (rbx != rsi) {
                    var_70 = r9;
                    var_68 = rcx;
                    runtime.memmove(r8);
                    rdx = var_50;
                    rsi = var_70;
                    rax = var_80;
                    rcx = var_68;
                    rdi = rdx;
                    r9 = rsi;
                    rdx = var_b0;
                }
                __asm("if (ja short 0xADDR /* unmodelled flags */) goto 0xADDR");
            }
        }
    }
        runtime.panicBounds(rcx, rdx, r8, r9);
    return;
}
