// L1 structure recovery (34 blocks)
void sub_140026c44_l1(void) {
    esi = 0;
    rdi = rcx;
    hstring2 = 0;
    var_30 = si;
    if (rcx == 0) {
        WindowsDeleteString(rcx);
        edx = 0;
        rcx = off_14002b210;
        WindowsCreateString(rcx, rdx, &hstring2);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            rdi = hstring2;
            rdx = off_14002a790;
            hstring2 = 0;
            var_30 = 1;
            sub_140001c90(0xADDR);
            rbx = rax;
            if (rax == 0) {
                rbx = 0;
            } else {
                rcx = qword_ADDR;
                *(u64*)(0) = "@)";
                *(u64*)(rbx) = off_140029578;
                *(u32*)((rbx + 0xADDR)) = 1;
                if (rcx != 0) {
                    rdx = *(u64*)(rcx);
                    rax = *(u64*)((rdx + 8));
                    sub_140028010(rcx, *(u64*)(rcx));
                }
                *(u64*)((rbx + 0xADDR)) = rdi;
                *(u64*)((rbx + 0xADDR)) = sub_14000ce80;
                rdi = rsi;
                *(u16*)((rbx + 0xADDR)) = si;
                *(u64*)(rbx) = off_1400294b0;
            }
            WindowsDeleteString(rdi);
            var_40 = rsi;
            hstring = rsi;
            WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, rsi);
            if (eax < 0 /* signed */) {
                ecx = eax;
                sub_14000b320(eax);
            }
            rcx = var_40;
            rdi = hstring;
            if (rcx != 0) {
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            RoGetActivationFactory(rdi, off_14002e110, &var_40);
            edi = eax;
            if (eax >= 0 /* signed */) {
                if (rbx == 0) {
                    edi = 0xADDR;
                } else {
                    rcx = var_40;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(var_40, rbx, off_140036400);
                    edi = eax;
                }
                rcx = var_40;
                if (rcx != 0) {
                    var_40 = rsi;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                if (rbx != 0) {
                    rax = *(u64*)(rbx);
                    rcx = rbx;
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                WindowsDeleteString(hstring2);
                eax = edi;
            } else {
                r9d = eax;
                sub_140007cf0(var_68, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                rcx = var_40;
                if (rcx != 0) {
                    var_40 = rsi;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                if (rbx != 0) {
                    rax = *(u64*)(rbx);
                    rcx = rbx;
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                ebx = edi;
                WindowsDeleteString(hstring2);
                eax = ebx;
            }
            r11 = &var_60;
            return;
        }
        r9d = ebx;
        sub_140007cf0(var_68, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        WindowsDeleteString(hstring2);
        eax = ebx;
        r11 = &var_60;
        return;
    }
    rbx = -1;
    do {
        rbx += 1;
    } while (*(u16*)((rcx + (rbx * 2))) != si);
    if (rbx > 0xADDR) {
        ebx = 0xADDR;
        r9d = 0xADDR;
        sub_140007cf0(var_68, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        WindowsDeleteString(hstring2);
        eax = 0xADDR;
        r11 = &var_60;
        return;
    }
    WindowsDeleteString(0);
    edx = ebx;
    hstring2 = rsi;
    rcx = rdi;
    WindowsCreateString(rcx, rdx, &hstring2);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rdi = hstring2;
        rdx = off_14002a790;
        hstring2 = rsi;
        var_30 = 1;
        sub_140001c90(0xADDR);
        rbx = rax;
        if (rax == 0) {
            rbx = rsi;
        } else {
            rcx = qword_ADDR;
            *(u64*)(rbx) = "@)";
            *(u64*)(rbx) = off_140029578;
            *(u32*)((rbx + 0xADDR)) = 1;
            if (rcx != 0) {
                rdx = *(u64*)(rcx);
                rax = *(u64*)((rdx + 8));
                sub_140028010(rcx, *(u64*)(rcx));
            }
            *(u64*)((rbx + 0xADDR)) = rdi;
            *(u64*)((rbx + 0xADDR)) = sub_14000ce80;
            rdi = rsi;
            *(u16*)((rbx + 0xADDR)) = si;
            *(u64*)(rbx) = off_1400294b0;
        }
        WindowsDeleteString(rdi);
        var_40 = rsi;
        hstring = rsi;
        WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, rsi);
        if (eax < 0 /* signed */) {
            ecx = eax;
            sub_14000b320(eax);
        }
        rcx = var_40;
        rdi = hstring;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
        RoGetActivationFactory(rdi, off_14002e110, &var_40);
        edi = eax;
        if (eax >= 0 /* signed */) {
            if (rbx == 0) {
                edi = 0xADDR;
            } else {
                rcx = var_40;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(var_40, rbx, off_140036400);
                edi = eax;
            }
            rcx = var_40;
            if (rcx != 0) {
                var_40 = rsi;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            if (rbx != 0) {
                rax = *(u64*)(rbx);
                rcx = rbx;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            WindowsDeleteString(hstring2);
            eax = edi;
        } else {
            r9d = eax;
            sub_140007cf0(var_68, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
            rcx = var_40;
            if (rcx != 0) {
                var_40 = rsi;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            if (rbx != 0) {
                rax = *(u64*)(rbx);
                rcx = rbx;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            ebx = edi;
            WindowsDeleteString(hstring2);
            eax = ebx;
        }
        r11 = &var_60;
        return;
    }
    r9d = 0xADDR;
    sub_140007cf0(var_68, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    WindowsDeleteString(hstring2);
    eax = 0xADDR;
    r11 = &var_60;
    return;
}
