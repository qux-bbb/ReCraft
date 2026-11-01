// L1 structure recovery (34 blocks)
void sub_14000bb98_l1(void) {
    var_20 = 0;
    rdi = rdx;
    r15 = rcx;
    edx = 0;
    sub_1400029dc(&var_40, 0, 0xADDR);
    var_28 = 0;
    sub_14000c120(rdi, &var_40, 0xADDR, &var_28);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rax = var_28;
        if (rax > rsi) {
            while (1) {
                edx = 0;
                rsi = rax;
                sub_140009e44(&var_30, 0, (rax - 1));
                sub_14000a0c0(&var_20, rax);
                rcx = var_30;
                if (rcx != 0) {
                    CoTaskMemFree(var_30);
                }
                rbx = var_20;
                if (rbx == 0) break;
                sub_14000c120(rdi, rbx, rsi, &var_28);
                if (eax >= 0 /* signed */) {
                    rax = var_28;
                    if (rax > rsi) continue;
                }
            }
            r9d = 0xADDR;
            sub_140007cf0(var_178, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource/wil/win32_helpers.h");
            if (rbx != 0) {
                CoTaskMemFree(rbx);
            }
            rcx = *(u64*)((rdi + 0xADDR));
            if (rcx != 0) {
                rdx = *(u64*)(rcx);
                rax = *(u64*)((rdx + 0xADDR));
                sub_140028010(rcx);
            }
            eax = 0xADDR;
        } else {
            sub_140009e44(&var_30, &var_40, (rax - 1));
            sub_14000a0c0(&var_20, rax);
            rcx = var_30;
            if (rcx != 0) {
                CoTaskMemFree(var_30);
            }
            rbx = var_20;
            if (rbx != 0) {
                var_20 = rbx;
                sub_14000a0c0(r15, &var_20);
                rcx = var_20;
                if (rcx != 0) {
                    CoTaskMemFree(var_20);
                }
                rcx = *(u64*)((rdi + 0xADDR));
                if (rcx != 0) {
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx);
                }
                eax = 0;
            } else {
                r9d = 0xADDR;
                sub_140007cf0(var_178, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource/wil/win32_helpers.h");
                if (rbx != 0) {
                    CoTaskMemFree(rbx);
                }
                rcx = *(u64*)((rdi + 0xADDR));
                if (rcx == 0) {
                    eax = 0xADDR;
                    return;
                }
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx);
                eax = 0xADDR;
                return;
            }
        }
    } else {
        rdx = *(u64*)((rdi + 0xADDR));
        if (rdx != 0) {
            rcx = *(u64*)(rdx);
            rax = *(u64*)((rcx + 0xADDR));
            rcx = rdx;
            sub_140028010(rdx);
        }
        eax = ebx;
    }
    return;
}
