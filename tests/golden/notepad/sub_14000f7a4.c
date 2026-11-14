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
        sub_140028010(rdi, "txt");
        ebx = eax;
        if (eax >= 0 /* signed */) {
            var_28 = qword_ADDR;
            var_20 = "*.txt";
            var_18 = qword_ADDR;
            var_10 = "*.*";
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
                            var_30 = 0;
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rdi, &var_30);
                            ebx = eax;
                            if (eax >= 0 /* signed */) {
                                rcx = var_30;
                                r8 = r14;
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(var_30, 0xADDR);
                                ebx = eax;
                            }
                            rcx = var_30;
                            if (rcx != 0) {
                                var_30 = 0;
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
    eax = ebx;
    return;
}
