// L1 structure recovery (21 blocks)
void sub_1400256d8_l1(void) {
    rsi = rdx;
    r14d = 0;
    var_20 = 0;
    sub_1400258d4(rcx, &var_20);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_20;
        var_28 = r14;
        rax = *(u64*)(rcx);
        rax = *(u64*)(rax);
        sub_140028010(var_20, off_14002af98, &var_28);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            var_30 = r14;
            hstring = r14;
            WindowsCreateStringReference("Windows.Security.EnterpriseData.FileProtectionManager", 0xADDR, &hstringHeader, &hstring);
            if (eax < 0 /* signed */) {
                ecx = eax;
                sub_14000b320(eax);
            }
            RoGetActivationFactory(hstring, off_14002e2b0, &var_30);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rcx = var_30;
                var_38 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(var_30, var_28, &var_38);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rdi = var_38;
                    sub_140024ebc(rdi, rdx, r8);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        rax = *(u64*)(rdi);
                        rdx = rsi;
                        rcx = rdi;
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rcx, rdx, r8);
                        ebx = eax;
                    }
                } else {
                    r9d = eax;
                    sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                }
                rcx = var_38;
                if (rcx != 0) {
                    var_38 = r14;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
            } else {
                r9d = eax;
                sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
            }
            rcx = var_30;
            if (rcx != 0) {
                var_30 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            rcx = var_28;
            if (rcx != 0) {
                var_28 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            rcx = var_20;
            if (rcx != 0) {
                var_20 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            eax = ebx;
            r11 = &var_70;
            return;
        }
        r9d = eax;
        sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        rcx = var_28;
        if (rcx != 0) {
            var_28 = r14;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
        rcx = var_20;
        if (rcx != 0) {
            var_20 = r14;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
        eax = ebx;
        r11 = &var_70;
        return;
    }
    r9d = eax;
    sub_140007cf0(var_88, (r14 + 0xADDR), "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    rcx = var_20;
    if (rcx != 0) {
        var_20 = r14;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    eax = ebx;
    r11 = &var_70;
    return;
}
