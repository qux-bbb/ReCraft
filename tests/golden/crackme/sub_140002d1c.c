// L1 structure recovery (51 blocks)
void sub_140002d1c_l1(void) {
    r14 = r9;
    rbx = r8;
    rsi = rdx;
    r13 = rcx;
    edi = 0;
    r15 = (int64)(*(u32*)((r8 + 4)));
    if (r15d == 0) {
        rdx = rdi;
    } else {
        sub_140002b30(rcx);
        rdx = (r15 + rax);
    }
    if (rdx == 0) {
        eax = 0;
        return;
    }
    r15 = (int64)(*(u32*)((rbx + 4)));
    if (r15d == 0) {
        rcx = rdi;
    } else {
        sub_140002b30(rcx);
        rcx = (r15 + rax);
    }
    if (*(u8*)((rcx + 0xADDR)) == dil) {
        eax = 0;
        return;
    }
    if (*(u32*)((rbx + 8)) != edi) {
        if (*(u32*)(rbx) >= edi /* signed */) {
            rax = (int64)(*(u32*)((rbx + 8)));
            rax += *(u64*)(rsi);
            rsi = rax;
        }
        if ((*(u8*)(rbx) & 0xADDR) == 0) {
            loc_ADDR:
            if ((*(u8*)(rbx) & 8) == 0) {
                if ((*(u8*)(r14) & 1) == 0) {
                    rbx = (int64)(*(u32*)((r14 + 0xADDR)));
                    if (ebx == 0) {
                        rcx = rdi;
                    } else {
                        sub_140002b44(rcx);
                        rcx = (rbx + rax);
                    }
                    if (rcx != 0) {
                        if (*(u64*)((r13 + 0xADDR)) == rdi) {
                            abort();
                            abort();
                        }
                        if (rsi == 0) {
                            abort();
                            abort();
                            return;
                        }
                        rbx = (int64)(*(u32*)((r14 + 0xADDR)));
                        if (ebx == 0) {
                            rcx = rdi;
                        } else {
                            sub_140002b44(rcx);
                            rcx = (rbx + rax);
                        }
                        if (rcx == 0) {
                            abort();
                            abort();
                            return;
                        }
                        al = (*(u8*)(r14) & 4);
                        al = -(al);
                        ecx = (((al != 0) ? 1 : 0) + 1);
                        edi = ecx;
                        var_20 = ecx;
                        eax = edi;
                        return;
                    }
                    if (*(u64*)((r13 + 0xADDR)) == rdi) {
                        abort();
                    }
                    if (rsi == 0) {
                        abort();
                        abort();
                        abort();
                        return;
                    }
                    rbx = (int64)(*(u32*)((r14 + 0xADDR)));
                    sub_140001ffc(*(u64*)((r13 + 0xADDR)), (r14 + 8));
                    memcpy(rsi, rax, rbx);
                    eax = edi;
                    return;
                }
                rdx = *(u64*)((r13 + 0xADDR));
                if (rdx == 0) {
                    abort();
                }
                if (rsi == 0) {
                    abort();
                    abort();
                    abort();
                    abort();
                    return;
                }
                memcpy(rsi, rdx, (int64)(*(u32*)((r14 + 0xADDR))));
                if (*(u32*)((r14 + 0xADDR)) != 8) {
                    eax = edi;
                    return;
                }
                if (*(u64*)(rsi) == rdi) {
                    eax = edi;
                    return;
                }
                rcx = *(u64*)(rsi);
                sub_140001ffc(rcx, (r14 + 8));
                *(u64*)(rsi) = rax;
                eax = edi;
                return;
            }
            rcx = *(u64*)((r13 + 0xADDR));
            if (rcx == 0) {
                abort();
            }
            if (rsi == 0) {
                abort();
                abort();
                abort();
                abort();
                abort();
                return;
            }
            *(u64*)(rsi) = rcx;
            sub_140001ffc(rcx, (r14 + 8));
            *(u64*)(rsi) = rax;
            eax = edi;
            return;
        }
        if ((*(u8*)(r14) & 0xADDR) == 0) goto loc_ADDR;
    }
    if (*(u32*)(rbx) >= edi /* signed */) {
        eax = 0;
        return;
    }
    rax = qword_ADDR;
    if (rax == 0) goto loc_ADDR;
    (*qword_ADDR)();
    if ((rax == 0) || (rsi == 0)) {
        abort();
        *(u64*)(rsi) = rax;
        rcx = rax;
        sub_140001ffc(rcx, (r14 + 8));
        *(u64*)(rsi) = rax;
        eax = edi;
        return;
    }
    *(u64*)(rsi) = rax;
    rcx = rax;
    sub_140001ffc(rcx, (r14 + 8));
    *(u64*)(rsi) = rax;
    eax = edi;
    return;
}
