// L1 structure recovery (30 blocks)
void sub_140024ebc_l1(void) {
    var_70 = r8;
    var_68 = edx;
    rsi = rcx;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 8));
        sub_140028010(rcx, rdx);
    }
    rdx = off_14002a790;
    sub_140001c90(0xADDR);
    rbx = rax;
    if (rax != 0) {
        rax = "@)";
        rdi = (rbx + 8);
        *(u64*)(rbx) = rax;
        sub_140009f6c(rdi);
        rcx = qword_ADDR;
        *(u64*)(rbx) = 0xADDR;
        *(u64*)(rdi) = 0xADDR;
        *(u32*)((rbx + 0xADDR)) = 1;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 8));
            sub_140028010(rcx, rdx);
        }
        *(u32*)((rbx + 0xADDR)) = 0;
        *(u64*)((rbx + 0xADDR)) = 0;
        *(u64*)(rbx) = 0xADDR;
        rax = 0xADDR;
        *(u64*)(rdi) = 0xADDR;
        CreateEventExW(0, 0, 0, 0xADDR);
        *(u64*)((rbx + 0xADDR)) = 0xADDR;
        if (rax != 0) {
            rax = *(u64*)(rbx);
            rax = *(u64*)((rax + 8));
            rcx = rbx;
            sub_140028010(rbx);
            rax = *(u64*)(rbx);
            rcx = rbx;
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx);
            rax = *(u64*)(rsi);
            rdx = rbx;
            rcx = rsi;
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx);
            var_68 = eax;
            rcx = rbx;
            if (eax >= 0 /* signed */) {
                rax = *(u64*)((rbx + 0xADDR));
                var_8 = 0;
                r8d = 1;
                var_70 = 0;
                edx = -1;
                var_10 = rax;
                rax = &var_70;
                var_20 = rax;
                CoWaitForMultipleHandles((r8 + 7), (edx | -1), 1, &var_10, &var_70);
                var_68 = eax;
                if (eax < 0 /* signed */) {
                    var_60 = 0;
                } else if (var_70 != 0) {
                    rax = *(u64*)(rsi);
                    var_60 = 0;
                    var_68 = 0xADDR;
                    rdi = *(u64*)(rax);
                    sub_140009b98(&var_60);
                    r8 = rax;
                    rax = rdi;
                    sub_140028010(rsi, "6");
                    if (eax >= 0 /* signed */) {
                        rcx = var_60;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(var_60);
                    }
                }
                if (var_68 >= 0 /* signed */ && *(u32*)((rbx + 0xADDR)) != 1) {
                    if (var_60 != 0) {
                        rcx = var_60;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(var_60, &var_68);
                    } else {
                        rax = *(u64*)(rsi);
                        rdi = *(u64*)(rax);
                        sub_140009b98(&var_60);
                        r8 = rax;
                        rax = rdi;
                        sub_140028010(rsi, "6");
                    }
                }
                rdx = var_60;
                rcx = rbx;
                if (rdx != 0) {
                    var_60 = 0;
                    rcx = rdx;
                    rax = *(u64*)(rdx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx);
                    rcx = rbx;
                }
            }
            rax = *(u64*)(rcx);
            edi = var_68;
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx);
        } else {
            GetLastError();
            edi = eax;
            if (eax > 0 /* signed */) {
                edi = (uint32)(ax);
                edi |= 0xADDR;
            }
            rax = *(u64*)(rbx);
            if (edi < 0 /* signed */) {
                rax = *(u64*)((rax + 0xADDR));
                rcx = rbx;
                sub_140028010(rbx);
                var_68 = edi;
            }
        }
    } else {
        var_68 = 0xADDR;
    }
    if (rsi != 0) {
        rcx = *(u64*)(rsi);
        rax = *(u64*)((rcx + 0xADDR));
        rcx = rsi;
        sub_140028010(rsi);
    }
    eax = edi;
    return;
}
