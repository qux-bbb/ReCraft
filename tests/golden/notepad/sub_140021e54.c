// L1 structure recovery (22 blocks)
void sub_140021e54_l1(void) {
    var_80 = r8;
    rbp = rdx;
    r12 = &var_80;
    r14 = rcx;
    r15d = 0;
    var_28 = 0;
    sub_140002a5c(rbp, r12);
    rdi = (int64)(eax);
    var_20 = 0;
    edx = 0;
    sub_140009e44(&var_28, 0, rdi);
    sub_14000a0c0(&var_20, rax);
    rcx = var_28;
    if (rcx != 0) {
        CoTaskMemFree(var_28);
    }
    rbx = var_20;
    if (rbx != 0) {
        rsi = (rdi + 1);
        rax = (rsi - 1);
        if (rax <= 0xADDR) {
            sub_1400029f4(rbx, rdi, rbp, r12);
            if (eax < 0 /* signed */) {
                rax = 0xADDR;
                *(u16*)((rbx + (rdi * 2))) = r15w;
                edi = 0xADDR;
                __asm("if (jbe short 0xADDR /* unmodelled flags */) goto 0xADDR");
                *(u16*)(rbx) = r15w;
            } else {
                rax = (int64)(eax);
                if (rax <= 0xADDR) {
                    if (rax == rdi) {
                        *(u16*)((rbx + (rdi * 2))) = r15w;
                    }
                    edi = r15d;
                }
            }
            if (edi >= 0 /* signed */) {
                var_20 = rbx;
                sub_14000a0c0(r14, &var_20);
                rcx = var_20;
                if (rcx != 0) {
                    CoTaskMemFree(var_20);
                }
                edi = r15d;
            } else {
                edx = 0xADDR;
                r9d = edi;
                sub_140007cf0(var_68, rdx, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\resource.h");
                if (rbx != 0) {
                    CoTaskMemFree(rbx);
                }
            }
        } else {
            edi = 0xADDR;
            if (rsi == 0) {
                edx = 0xADDR;
                r9d = edi;
                sub_140007cf0(var_68, rdx, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\resource.h");
                if (rbx != 0) {
                    CoTaskMemFree(rbx);
                }
                eax = edi;
                return;
            }
        }
    } else {
        edi = 0xADDR;
        edx = 0xADDR;
    }
    eax = edi;
    return;
}
