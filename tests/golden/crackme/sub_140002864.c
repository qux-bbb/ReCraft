// L1 structure recovery (17 blocks)
void sub_140002864_l1(void) {
    rbx = var_90;
    r15 = rdx;
    rsi = rcx;
    rdx = r9;
    rdi = r9;
    r14d = r8d;
    ebp = *(u32*)((rbx + 0xADDR));
    sub_140002cac(rbx);
    r10d = 0;
    r9d = eax;
    if (ebp == 0) {
        abort();
    }
    rdi = *(u64*)((rdi + 8));
    r12d = -1;
    r11 = (int64)(*(u32*)((rbx + 0xADDR)));
    r8d = -1;
    r13d = -1;
    edx = ebp;
    while (1) {
        eax = (rdx - 1);
        rcx = (rax + (rax * 4));
        ebx = rdx;
        edx = eax;
        rax = (rdi + (rcx * 4));
        if (r9d <= *(u32*)(((rax + r11) + 4)) /* signed */) {
            if (edx != 0) continue;
        } else {
            if (r9d <= *(u32*)(((rax + r11) + 8)) /* signed */) break;
        }
    }
    eax = (rbx - 1);
    rax += (rax * 4);
    r9 = (rdi + (rax * 4));
    r9 += r11;
    edx = r10d;
    rcx = (rdi + r11);
    do {
        if (r9 == 0) {
            if (r14d >= *(u32*)(rcx) /* signed */ && r14d <= *(u32*)((rcx + 4)) /* signed */) {
                r13d = edx;
                r8d = ((r8d == r12d) ? edx : r8d);
            }
        } else {
            eax = *(u32*)((r9 + 4));
            if (*(u32*)(rcx) > eax /* signed */) {
                eax = *(u32*)((r9 + 8));
            }
        }
        edx += 1;
        rcx += 0xADDR;
    } while (edx < ebp);
    eax = r10d;
    var_20 = r15;
    r11 = &var_40;
    var_30 = r15;
    rbx = *(u64*)((r11 + 0xADDR));
    rbp = *(u64*)((r11 + 0xADDR));
    eax = ((r8d != r12d) ? r8d : eax);
    var_28 = eax;
    eax = (r13 + 1);
    xmm0 = var_20;
    r10d = ((r8d != r12d) ? eax : r10d);
    rax = rsi;
    var_38 = r10d;
    xmm1 = var_30;
    *(__m128*)(rsi) = xmm0;
    *(__m128*)((rsi + 0xADDR)) = xmm1;
    return;
}
