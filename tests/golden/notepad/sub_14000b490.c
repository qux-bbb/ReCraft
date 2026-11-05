// L1 structure recovery (49 blocks)
void sub_14000b490_l1(void) {
    edx = 0;
    sub_1400029dc(&var_80, 0, 0xADDR);
    sub_140023b80(&var_80, 0xADDR);
    PathFileExistsW(&var_80);
    esi = 0;
    if (eax == 0) {
        var_30 = 0;
        hstring = 0;
        WindowsCreateStringReference("Windows.System.Launcher", 0xADDR, &hstringHeader, &hstring);
        if (eax < 0 /* signed */) {
            ecx = eax;
            sub_14000b320(eax);
        }
        rcx = var_30;
        rbx = hstring;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8, r9);
        }
        RoGetActivationFactory(rbx, off_14002af50, &var_30);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            var_38 = 0;
            hstring = rsi;
            WindowsCreateStringReference("W", 0xADDR, &hstringHeader, &hstring);
            if (eax < 0 /* signed */) {
                ecx = eax;
                sub_14000b320(eax);
            }
            rcx = var_38;
            rbx = hstring;
            if (rcx != 0) {
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8, r9);
            }
            RoGetActivationFactory(rbx, off_14002afe8, &var_38);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rbx = var_38;
                var_40 = rsi;
                rax = *(u64*)(rbx);
                rdi = *(u64*)((rax + 0xADDR));
                sub_140009b18(&hstringHeader, off_1400295a0);
                rdx = *(u64*)((rax + 0xADDR));
                rax = rdi;
                sub_140028010(rbx, *(u64*)((rax + 0xADDR)), &var_40);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rcx = var_30;
                    r8d = 0;
                    var_48 = rsi;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(var_30, var_40, 0, &var_48);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        rdi = var_48;
                        var_50 = 4;
                        sub_140009bcc(rdi, rdx, r8, r9);
                        ebx = eax;
                        if (eax < 0 /* signed */) {
                            r9d = ebx;
                            sub_140007cf0(var_2a8, 0xADDR, "shell\\osshell\\accesory\\notepad\\banner.cpp");
                            rcx = var_48;
                            if (rcx != 0) {
                                var_48 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_40;
                            if (rcx != 0) {
                                var_40 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_38;
                            if (rcx != 0) {
                                var_38 = rsi;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            rcx = var_30;
                            if (rcx != 0) {
                                var_30 = rsi;
                                rdx = *(u64*)(rcx);
                                rax = *(u64*)((rdx + 0xADDR));
                                sub_140028010(rcx, rdx, r8, r9);
                            }
                            eax = ebx;
                        } else {
                            rax = *(u64*)(rdi);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rdi, &var_50);
                            ebx = eax;
                            if (eax >= 0 /* signed */) {
                                if (var_50 == esi) {
                                    PostMessageW(qword_ADDR, 0xADDR, 0, 0);
                                }
                                rcx = var_48;
                                if (rcx != 0) {
                                    var_48 = rsi;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(rcx, rdx, r8, r9);
                                }
                                rcx = var_40;
                                if (rcx != 0) {
                                    var_40 = rsi;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(rcx, rdx, r8, r9);
                                }
                                rcx = var_38;
                                if (rcx != 0) {
                                    var_38 = rsi;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(rcx, rdx, r8, r9);
                                }
                                rcx = var_30;
                                if (rcx != 0) {
                                    var_30 = rsi;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(rcx, rdx, r8, r9);
                                }
                                eax = 0;
                            }
                        }
                    } else {
                        r9d = eax;
                        sub_140007cf0(var_2a8, 0xADDR, "shell\\osshell\\accesory\\notepad\\banner.cpp");
                        rcx = var_48;
                        if (rcx != 0) {
                            var_48 = rsi;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8, r9);
                        }
                        rcx = var_40;
                        if (rcx != 0) {
                            var_40 = rsi;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8, r9);
                        }
                        rcx = var_38;
                        if (rcx != 0) {
                            var_38 = rsi;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rcx, rdx, r8, r9);
                        }
                        rcx = var_30;
                        if (rcx == 0) {
                            eax = ebx;
                            r11 = &var_2a0;
                            return;
                        }
                        var_30 = rsi;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                    }
                } else {
                    r9d = eax;
                    sub_140007cf0(var_2a8, 0xADDR, "shell\\osshell\\accesory\\notepad\\banner.cpp");
                }
            } else {
                r9d = eax;
                sub_140007cf0(var_2a8, 0xADDR, "shell\\osshell\\accesory\\notepad\\banner.cpp");
            }
            r11 = &var_2a0;
            return;
        }
        r9d = eax;
        sub_140007cf0(var_2a8, 0xADDR, "shell\\osshell\\accesory\\notepad\\banner.cpp");
    }
    PostMessageW(qword_ADDR, 0xADDR, 0, (rsi + 1));
    eax = 0;
    r11 = &var_2a0;
    return;
}
