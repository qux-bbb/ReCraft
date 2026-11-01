// L1 structure recovery (13 blocks)
void sub_14000d54c_l1(void) {
    var_30 = 0;
    rax = &var_30;
    rdi = rdx;
    edx = 0;
    rsi = rcx;
    CoCreateInstance(off_14002a850, 0, (rdx + 1), off_14002b258, &var_30);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_30;
        var_78 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)(rax);
        sub_140028010(var_30, off_14002b248, &var_78);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            sub_14000c1e4(var_78, 1);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rcx = var_78;
                r8d = 0;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(var_78, (r8 + 1));
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    sub_14000f7a4(rsi, var_30, qword_ADDR, rdi);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        rcx = var_78;
                        var_70 = 0;
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(var_78, 1, &var_70);
                        ebx = eax;
                        if (eax >= 0 /* signed */) {
                            eax = var_70;
                            dword_ADDR = eax;
                            sub_140022f54(rcx);
                            if (dword_ADDR == 0) {
                                sub_14000ca10(*(u64*)(rdi));
                                dword_ADDR = eax;
                            }
                        }
                    }
                }
            }
        }
        rcx = var_78;
        if (rcx != 0) {
            var_78 = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
    }
    rcx = var_30;
    if (rcx != 0) {
        var_30 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    eax = ebx;
    return;
}
