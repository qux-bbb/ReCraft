// L1 structure recovery (24 blocks)
void sub_14000c3a4_l1(void) {
    r12d = 0;
    rdi = r9;
    rbx = r8;
    r14 = rdx;
    r15 = rcx;
    ebp = 0xADDR;
    if (r9 == 0) {
        esi = 0;
    } else {
        wcsnlen(r9, ebp);
        rsi = rax;
    }
    if (rbx != 0) {
        wcsnlen(rbx, rbp);
    } else {
        eax = 0;
    }
    esi += eax;
    edx = (rsi + 1);
    rdx += rdx;
    LocalAlloc(0xADDR, (rdx + rdx));
    rbp = rax;
    if (rax == 0) {
        MessageBoxW(r15, rbx, r14, var_60);
        ebx = eax;
    } else {
        r8d = (uint32)(word_ADDR);
        edx = 0;
        rcx = rax;
        while (*(u16*)(rbx) != r8w) {
            if (edx >= esi) break;
            edx += 1;
            *(u16*)(rcx) = ax;
            rcx += 2;
            eax = (uint32)(*(u16*)(rbx));
            rbx += 2;
            if (ax != 0) continue;
        }
        if (rdi != 0 && *(u16*)(rdi) != r12w) {
            rdi -= rcx;
            while (edx < esi) {
                eax = (uint32)(*(u16*)((rdi + rcx)));
                edx += 1;
                *(u16*)(rcx) = ax;
                rcx += 2;
                if (*(u16*)((rdi + rcx)) == r12w) break;
            }
        }
        rbx += 4;
        while (1) {
            eax = (uint32)(*(u16*)(rbx));
            if (ax == 0) break;
            if (edx >= esi) break;
            *(u16*)(rcx) = ax;
            rbx += 2;
            rcx += 2;
            edx += 1;
        }
        *(u16*)(rcx) = r12w;
        MessageBoxW(r15, rbp, r14, var_60);
        ebx = eax;
        LocalFree(rbp);
    }
    eax = ebx;
    return;
}
