// L1 structure recovery (27 blocks)
void sub_140026268_l1(void) {
    var_60 = r8d;
    r15d = 0;
    rsi = r9;
    *(u32*)(r9) = 0;
    r14 = rdx;
    *(u16*)(rdx) = r15w;
    rbx = rcx;
    sub_140026b60();
    if (eax == 0) {
        *(u32*)(rsi) = 5;
        eax = 0;
    } else {
        var_58 = r15;
        sub_1400256d8(rbx, &var_58);
        ebx = 0;
        if (eax >= 0 /* signed */) {
            rcx = var_58;
            var_60 = r15d;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(var_58, &var_60);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                edx = var_60;
                rcx = r15;
                hstring = rcx;
                switch (edx) {
                    case 0xADDR: {
                        edx -= 0xADDR;
                        loc_ADDR:
                        *(u32*)(rsi) = 1;
                        WindowsDeleteString(rcx);
                        rcx = var_58;
                        hstring = r15;
                        if (rcx == 0) {
                            eax = 0;
                            return;
                        }
                        var_58 = r15;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rcx, rdx);
                        eax = 0;
                        return;
                    }
                    case 0xADDR: {
                        edx -= 0xADDR;
                        *(u32*)(rsi) = 3;
                        WindowsDeleteString(rcx);
                        rcx = var_58;
                        hstring = r15;
                        if (rcx == 0) {
                            eax = 0;
                            return;
                        }
                        var_58 = r15;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rcx, rdx);
                        eax = 0;
                        return;
                    }
                    case 0xADDR: {
                        edx -= 0xADDR;
                        rbx = var_58;
                        rax = *(u64*)(rbx);
                        rdi = *(u64*)((rax + 0xADDR));
                        WindowsDeleteString(r15);
                        rax = rdi;
                        sub_140028010(rbx, &hstring);
                        ebx = eax;
                        if (eax >= 0 /* signed */) {
                            WindowsGetStringRawBuffer(hstring, 0);
                            r8 = rax;
                            sub_140008384(r14, 0xADDR);
                            ebx = eax;
                            if (eax >= 0 /* signed */) {
                                rcx = hstring;
                                *(u32*)(rsi) = 2;
                                WindowsDeleteString(rcx);
                                rcx = var_58;
                                hstring = r15;
                                if (rcx == 0) {
                                    eax = 0;
                                    return;
                                }
                                var_58 = r15;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx);
                                eax = 0;
                                return;
                            }
                            edx = 0xADDR;
                            r9d = ebx;
                            sub_140007cf0(var_48, rdx, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                            WindowsDeleteString(hstring);
                            hstring = r15;
                            rcx = var_58;
                            if (rcx != 0) {
                                var_58 = r15;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx);
                            }
                            eax = ebx;
                        } else {
                            edx = 0xADDR;
                        }
                        break;
                    }
                    default: {
                        edx -= 0xADDR;
                        if (edx == 3) goto loc_ADDR;
                        break;
                    }
                }
            } else {
                edx = 0xADDR;
                r9d = ebx;
                sub_140007cf0(var_48, rdx, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
            }
        } else {
            edx = (r15 + 0xADDR);
        }
    }
    return;
}
