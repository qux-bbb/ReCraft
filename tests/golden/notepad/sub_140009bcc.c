// L1 structure recovery (24 blocks)
void sub_140009bcc_l1(void) {
    var_88 = r9;
    var_80 = r8d;
    var_78 = edx;
    r14 = rcx;
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
        rax = 0xADDR;
        *(u64*)(rdi) = rax;
        *(u32*)((rbx + 0xADDR)) = 1;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 8));
            sub_140028010(rcx, rdx);
        }
        *(u64*)(rbx) = 0xADDR;
        *(u64*)((rbx + 0xADDR)) = 0;
        rax = 0xADDR;
        *(u64*)(rdi) = rax;
        *(u32*)((rbx + 0xADDR)) = 0;
        CreateEventExW(0, 0, 0, 0xADDR);
        rsi = rax;
        if (rax == 0) {
            sub_1400057d8();
            edi = eax;
            if (eax >= 0 /* signed */) {
                rax = *(u64*)(rbx);
                rcx = rbx;
                rax = *(u64*)((rax + 8));
                sub_140028010(rcx, rdx);
                rax = *(u64*)(rbx);
                rcx = rbx;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx);
                rax = *(u64*)(r14);
                rdx = rbx;
                rcx = r14;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx);
                edi = eax;
                if (eax >= 0 /* signed */) {
                    rax = *(u64*)((rbx + 0xADDR));
                    var_78 = 0;
                    r8d = 1;
                    var_30 = rax;
                    edx = -1;
                    rax = &var_78;
                    var_20 = rax;
                    CoWaitForMultipleHandles((r8 + 7), (edx | -1), 1, &var_30, &var_78);
                    edi = eax;
                    if (eax >= 0 /* signed */) {
                        eax = *(u32*)((rbx + 0xADDR));
                        if (eax == 1) {
                            rax = *(u64*)(rbx);
                            rcx = rbx;
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx);
                            eax = 0;
                        } else {
                            rax = *(u64*)(r14);
                            var_88 = 0;
                            rdi = *(u64*)(rax);
                            sub_140009b98(&var_88);
                            r8 = rax;
                            rax = rdi;
                            sub_140028010(r14, "6");
                            edi = eax;
                            if (eax >= 0 /* signed */) {
                                rcx = var_88;
                                var_80 = 0xADDR;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(var_88, &var_80);
                                edi = eax;
                                edi = ((eax >= 0 /* signed */) ? var_80 : edi);
                            }
                            rcx = var_88;
                            if (rcx != 0) {
                                var_88 = 0;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx);
                            }
                            rax = *(u64*)(rbx);
                            rax = *(u64*)((rax + 0xADDR));
                            rcx = rbx;
                            sub_140028010(rbx);
                            eax = edi;
                        }
                    } else {
                        edx = 0xADDR;
                        r9d = edi;
                        sub_140007cf0(var_68, rdx, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\winrt.h");
                        rcx = *(u64*)(rbx);
                        rax = *(u64*)((rcx + 0xADDR));
                    }
                } else {
                    edx = 0xADDR;
                }
            } else {
                r9d = eax;
                sub_140007cf0(var_68, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\winrt.h");
                rcx = *(u64*)(rbx);
                rax = *(u64*)((rcx + 0xADDR));
                rcx = rbx;
                sub_140028010(rbx);
                r9d = edi;
                sub_140007cf0(var_68, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\winrt.h");
            }
        } else {
            GetLastError();
            sub_140009488((rbx + 0xADDR), rsi);
        }
    } else {
        edi = 0xADDR;
        r9d = edi;
        sub_140007cf0(var_68, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\winrt.h");
        eax = edi;
        return;
    }
    return;
}
