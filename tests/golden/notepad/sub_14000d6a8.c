// L1 structure recovery (27 blocks)
void sub_14000d6a8_l1(void) {
    r12d = 0;
    rax = &var_28;
    r14 = r9;
    var_28 = 0;
    rsi = r8;
    rdi = rdx;
    r15 = rcx;
    r13d = (r12 + 1);
    CoCreateInstance(off_14002a860, 0, r13d, off_14002b238, &var_28);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        *(u64*)(rsi) = 0;
        dword_ADDR = r13d;
        var_10 = 0;
        PSGetPropertyDescriptionListFromString("prop:System.Security.EncryptionOwners", off_14002b228, &var_10);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            rcx = var_28;
            r8d = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(var_28, var_10);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rcx = var_28;
                var_20 = r12;
                rax = *(u64*)(rcx);
                rax = *(u64*)(rax);
                sub_140028010(var_28, off_14002b248, &var_20);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    edx = 0;
                    sub_14000c1e4(var_20, 0);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        edx = dword_ADDR;
                        rcx = var_20;
                        r8d = dword_ADDR;
                        r8d = ((edx != 0) ? edx : r8d);
                        edx = r13d;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(var_20);
                        if (eax >= 0 /* signed */) {
                            rbx = var_28;
                            PathIsFileSpecW(rdi);
                            if (eax == 0) {
                                var_30 = r12;
                                SHCreateItemFromParsingName(rdi, 0, off_14002a840, &var_30);
                                if (eax >= 0 /* signed */) {
                                    rax = *(u64*)(rbx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(rbx, var_30);
                                    rcx = var_30;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(var_30);
                                }
                            } else {
                                rax = *(u64*)(rbx);
                                rdx = rdi;
                                rcx = rbx;
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8);
                            }
                            rcx = var_28;
                            var_18 = r12;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)(rax);
                            sub_140028010(var_28, off_14002b218, &var_18);
                            ebx = eax;
                            if (eax >= 0 /* signed */) {
                                sub_14000f7a4(r15, var_18, qword_ADDR, rsi);
                                ebx = eax;
                                if (eax >= 0 /* signed */) {
                                    rcx = var_20;
                                    var_30 = r12d;
                                    rax = *(u64*)(rcx);
                                    rax = *(u64*)((rax + 0xADDR));
                                    sub_140028010(var_20, r13d, &var_30);
                                    ebx = eax;
                                    if (eax >= 0 /* signed */) {
                                        dword_ADDR = var_30;
                                    }
                                }
                            }
                            rcx = var_18;
                            if (rcx != 0) {
                                var_18 = r12;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8);
                            }
                        } else {
                            rcx = var_20;
                            rax = *(u64*)(rcx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(var_20, r13d, (r12 + 5));
                            ebx = eax;
                        }
                    }
                }
                rcx = var_20;
                if (rcx != 0) {
                    var_20 = r12;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                if (ebx >= 0 /* signed */ && r14 != 0) {
                    rcx = var_28;
                    r8 = r14;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)(rax);
                    sub_140028010(var_28, off_14002b238);
                }
            }
        }
        rcx = var_10;
        dword_ADDR = r12d;
        if (rcx != 0) {
            var_10 = r12;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
    }
    rcx = var_28;
    if (rcx != 0) {
        var_28 = r12;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    eax = ebx;
    return;
}
