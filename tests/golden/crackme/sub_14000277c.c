// L1 structure recovery (12 blocks)
void sub_14000277c_l1(void) {
    ebp = *(u32*)((r8 + 0xADDR));
    r13 = rcx;
    rsi = r9;
    r15 = r8;
    r12 = rdx;
    sub_140002cac(r8);
    r10 = *(u64*)(r13);
    r14d = eax;
    *(u64*)(rsi) = r10;
    while (ebp != 0) {
        rax = (int64)(*(u32*)((r15 + 0xADDR)));
        ecx = &var_n1;
        rdi = *(u64*)((r12 + 8));
        ebp = ecx;
        rcx += (rcx * 4);
        rbx = (rax + (rcx * 4));
        if (r14d <= *(u32*)(((rbx + rdi) + 4)) /* signed */) continue;
    }
    rax = rsi;
    return;
    do {
        rax = (int64)(*(u32*)(rdx));
        if (rax == r10) {
            rax = *(u64*)(r13);
            rcx += (rcx * 4);
            rcx = (int64)(*(u32*)(((r8 + (rcx * 4)) + 0xADDR)));
            rcx = *(u64*)((rcx + rax));
            *(u64*)(rsi) = rcx;
            rax = rsi;
            return;
        }
        ecx += 1;
        rdx += 0xADDR;
    } while (ecx < r9d);
    rax = *(u64*)(r13);
    rcx += (rcx * 4);
    rcx = (int64)(*(u32*)(((r8 + (rcx * 4)) + 0xADDR)));
    rcx = *(u64*)((rcx + rax));
    *(u64*)(rsi) = rcx;
}
