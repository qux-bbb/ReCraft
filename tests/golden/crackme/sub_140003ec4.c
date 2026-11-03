// L1 structure recovery (22 blocks)
void sub_140003ec4_l1(void) {
    var_88 = r9d;
    r12d = r9d;
    rsi = r8;
    rbx = rdx;
    r15 = rcx;
    sub_140002b30(rcx);
    r13 = rax;
    var_28 = rax;
    sub_140002c10(r15, rbx, rsi);
    edi = eax;
    sub_14000217c(rcx);
    *(u32*)((rax + 0xADDR)) += 1;
    while (edi != -1) {
        if (edi <= r12d /* signed */) break;
        sub_140002b30(rcx);
        r14 = ((int64)(edi) << 3);
        rcx = (int64)(*(u32*)((rsi + 8)));
        rcx += r14;
        edi = *(u32*)((rcx + rax));
        var_20 = edi;
        rbx = (int64)(*(u32*)((rsi + 8)));
        sub_140002b30(rcx);
        rax += r14;
        if (*(u32*)(((rax + rbx) + 4)) == 0) {
            eax = 0;
        } else {
            rbx = (int64)(*(u32*)((rsi + 8)));
            sub_140002b30(rcx);
            rax += r14;
            rbx = (int64)(*(u32*)(((rax + rbx) + 4)));
            sub_140002b30(rcx);
            rax += rbx;
        }
        if (rax != 0) {
            r8d = edi;
            rdx = rsi;
            rcx = r15;
            sub_140002c64(r15, rsi, edi);
            rbx = (int64)(*(u32*)((rsi + 8)));
            sub_140002b30(rcx);
            rax += r14;
            if (*(u32*)(((rax + rbx) + 4)) == 0) {
                eax = 0;
            } else {
                rbx = (int64)(*(u32*)((rsi + 8)));
                sub_140002b30(rcx);
                rax += r14;
                rbx = (int64)(*(u32*)(((rax + rbx) + 4)));
                sub_140002b30(rcx);
                rax += rbx;
            }
            sub_1400042d0(rax, r15, 0xADDR);
            sub_140002b58(r13);
        }
        var_24 = edi;
        continue;
    }
    sub_14000217c(rcx);
    if (*(u32*)((rax + 0xADDR)) > 0 /* signed */) {
        sub_14000217c(rcx);
        *(u32*)((rax + 0xADDR)) -= 1;
    }
    if (edi == -1) {
        r8d = edi;
        rdx = rsi;
        rcx = r15;
        sub_140002c64(r15, rsi, edi);
        return;
    }
    if (edi > r12d /* signed */) {
        sub_140008860();
    }
    sub_140008860();
}
