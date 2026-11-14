// L1 structure recovery (26 blocks)
void sub_1400269b0_l1(void) {
    var_50 = 0;
    hstring = 0;
    var_40 = rcx;
    WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    rcx = var_50;
    rbx = hstring;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    RoGetActivationFactory(rbx, off_14002e1b0, &var_50);
    if (eax < 0 /* signed */) {
        rcx = var_50;
        if (rcx != 0) {
            var_50 = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
        eax = 4;
    } else {
        rbx = var_50;
        var_48 = 0;
        rax = *(u64*)(rbx);
        rdi = *(u64*)((rax + 0xADDR));
        sub_140009b18(&hstringHeader, &var_40);
        rdx = *(u64*)((rax + 0xADDR));
        rax = rdi;
        sub_140028010(rbx, *(u64*)((rax + 0xADDR)), &var_48);
        if (eax >= 0 /* signed */) {
            ecx = var_48;
            if (ecx == 0) {
                rcx = var_50;
                if (rcx != 0) {
                    var_50 = 0;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                eax = 0;
            } else {
                switch (ecx) {
                    case 0xADDR: {
                        ecx -= 0xADDR;
                        rcx = var_50;
                        if (rcx != 0) {
                            var_50 = 0;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8);
                        }
                        eax = 1;
                        break;
                    }
                    case 0xADDR: {
                        ecx -= 0xADDR;
                        rcx = var_50;
                        if (rcx != 0) {
                            var_50 = 0;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8);
                        }
                        eax = 2;
                        break;
                    }
                    case 0xADDR: {
                        ecx -= 0xADDR;
                        rcx = var_50;
                        if (rcx != 0) {
                            var_50 = 0;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8);
                        }
                        eax = 3;
                        break;
                    }
                }
            }
        }
    }
    return;
}
