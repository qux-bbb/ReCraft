// L1 structure recovery (11 blocks)
void sub_14000f7a4_l1(void) {
    rax = *(u64*)(rdx);
    rdi = rdx;
    rsi = rcx;
    rdx = r8;
    rcx = rdi;
    r14 = r9;
    rax = *(u64*)((rax + 0xADDR));
    sub_140028010(rcx, rdx, r8);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rax = *(u64*)(rdi);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rdi, "t");
        ebx = eax;
        if (eax >= 0 /* signed */) {
            var_28 = qword_ADDR;
            var_30 = "*.txt";
            var_38 = qword_ADDR;
            var_40 = "*";
            rax = *(u64*)(rdi);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rdi, 2, &var_28);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rax = *(u64*)(rdi);
                var_78 = 0;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rdi, &var_78);
                ebx = eax;
                if (eax >= 0 /* signed */) {
                    rax = *(u64*)(rdi);
                    rcx = rdi;
                    edx = (var_78 | 0xADDR);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                    ebx = eax;
                    if (eax >= 0 /* signed */) {
                        rax = *(u64*)(rdi);
                        rdx = rsi;
                        rcx = rdi;
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rcx, rdx, r8);
                        ebx = eax;
                        if (eax >= 0 /* signed */) {
                            rax = *(u64*)(rdi);
                            var_20 = 0;
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rdi, &var_20);
                            ebx = eax;
                            if (eax >= 0 /* signed */) {
                                rcx = var_20;
                                r8 = r14;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(var_20, 0xADDR);
                                ebx = eax;
                            }
                            rcx = var_20;
                            if (rcx != 0) {
                                var_20 = 0;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rcx, rdx, r8);
                            }
                        }
                    }
                }
            }
        }
    }
    r11 = &var_50;
    eax = ebx;
    return;
}
