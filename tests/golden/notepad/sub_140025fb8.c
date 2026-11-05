// L1 structure recovery (26 blocks)
void sub_140025fb8_l1(void) {
    r14d = 0;
    rsi = rcx;
    *(u16*)(rcx) = 0xADDR;
    var_20 = 0;
    hstring = 0;
    WindowsCreateStringReference("Windows.ApplicationModel.DataTransfer.Clipboard", (r14 + 0xADDR), &hstringHeader, &hstring, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    rcx = var_20;
    rbx = hstring;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    RoGetActivationFactory(rbx, off_14002e2c0, &var_20);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_20;
        var_28 = r14;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(var_20, &var_28);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            rcx = var_28;
            var_30 = r14;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(var_28, &var_30);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rcx = var_30;
                var_38 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)(rax);
                sub_140028010(var_30, off_14002e120, &var_38);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rbx = var_38;
                    rax = *(u64*)(rbx);
                    rdi = *(u64*)((rax + 0xADDR));
                    WindowsDeleteString(0);
                    hstring2 = r14;
                    rax = rdi;
                    sub_140028010(rbx, &hstring2);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        WindowsGetStringRawBuffer(hstring2, 0);
                        r8 = rax;
                        sub_140008384(rsi, 0xADDR);
                        ebx = eax;
                        WindowsDeleteString(hstring2);
                        rcx = var_38;
                        hstring2 = r14;
                        if (rcx != 0) {
                            var_38 = r14;
                            rax = *(u64*)((*(u64*)(rcx) + 0xADDR));
                            sub_140028010(rcx, rdx, r8);
                        }
                    } else {
                        r9d = eax;
                        sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                        WindowsDeleteString(hstring2);
                        hstring2 = r14;
                        rcx = var_38;
                        if (rcx != 0) {
                            var_38 = r14;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8);
                            eax = ebx;
                            r11 = &var_70;
                            return;
                        }
                    }
                } else {
                    r9d = eax;
                    sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
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
        } else {
            r9d = eax;
            sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        }
        rcx = var_28;
        if (rcx != 0) {
            var_28 = r14;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
    } else {
        r9d = eax;
        sub_140007cf0(var_88, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
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
