// L1 structure recovery (13 blocks)
void sub_14000c1e4_l1(void) {
    rax = *(u64*)(rcx);
    sil = dl;
    rbx = rcx;
    rax = *(u64*)((rax + 0xADDR));
    sub_140028010(rcx, 2, qword_ADDR);
    edi = eax;
    if (eax >= 0 /* signed */) {
        rax = *(u64*)(rbx);
        ebp = 1;
        edx = ebp;
        rcx = rbx;
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8, r9);
        edi = eax;
        if (eax >= 0 /* signed */) {
            if (sil == 0) {
                rax = *(u64*)(rbx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rbx, ebp, ebp, qword_ADDR);
                edi = eax;
                if (eax >= 0 /* signed */) {
                    rax = *(u64*)(rbx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rbx, ebp, 2, qword_ADDR);
                    edi = eax;
                    if (eax >= 0 /* signed */) {
                        rax = *(u64*)(rbx);
                        rax = *(u64*)((rax + 0xADDR));
                        sub_140028010(rbx, ebp, 3, qword_ADDR);
                        edi = eax;
                        if (eax >= 0 /* signed */) {
                            rax = *(u64*)(rbx);
                            rax = *(u64*)((rax + 0xADDR));
                            sub_140028010(rbx, ebp, 5, qword_ADDR);
                            edi = eax;
                            if (eax >= 0 /* signed */) {
                                rax = *(u64*)(rbx);
                                rax = *(u64*)((rax + 0xADDR));
                                sub_140028010(rbx, ebp, 4, qword_ADDR);
                                edi = eax;
                                if (eax >= 0 /* signed */) {
                                    GetACP();
                                    if (eax == 0xADDR) {
                                        rax = *(u64*)(rbx);
                                        rax = *(u64*)((rax + 0xADDR));
                                        sub_140028010(rbx, ebp, 6, "GB18030");
                                        edi = eax;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                rax = *(u64*)(rbx);
                r8d = 0;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rbx, ebp, 0, qword_ADDR);
                edi = eax;
            }
        }
        rax = *(u64*)(rbx);
        rcx = rbx;
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8, r9);
    }
    eax = edi;
    return;
}
