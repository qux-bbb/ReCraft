// L1 structure recovery (11 blocks)
void sub_140003c40_l1(void) {
    rdi = rcx;
    r15d = 0;
    var_20 = 0;
    var_b0 = 0;
    var_28 = 0;
    var_c8 = r15;
    sub_14000217c(rcx);
    r13 = *(u64*)((rax + 0xADDR));
    var_40 = r13;
    sub_14000217c(rcx);
    rax = *(u64*)((rax + 0xADDR));
    var_c0 = rax;
    rsi = *(u64*)((rdi + 0xADDR));
    var_58 = rsi;
    var_b8 = rsi;
    rbx = *(u64*)((rdi + 0xADDR));
    rax = *(u64*)((rdi + 0xADDR));
    var_48 = rax;
    r14 = *(u64*)((rdi + 0xADDR));
    var_50 = r14;
    sub_140002124(rbx);
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rsi;
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    sub_14000217c(rcx);
    rdx = *(u64*)((rax + 0xADDR));
    rdx = *(u64*)((rdx + 0xADDR));
    sub_140002aa0(&var_60, rdx);
    r12 = rax;
    var_38 = rax;
    if (*(u64*)((rdi + 0xADDR)) != r15) {
        var_b0 = 1;
        sub_14000217c(rcx);
        var_c8 = *(u64*)((rax + 0xADDR));
    }
    sub_1400042d0(var_48, r14, 0xADDR);
    rbx = rax;
    var_28 = rax;
    rdi = var_c0;
    sub_140002adc(r12);
    if (r15d == 0 && *(u32*)(rsi) == 0xADDR && *(u32*)((rsi + 0xADDR)) == 4) {
        eax = *(u32*)((rsi + 0xADDR));
        if (eax == 0xADDR) {
            sub_140001fcc(*(u64*)((rsi + 0xADDR)));
            if (eax != 0) {
                sub_140001f54(rsi, 1);
            }
        } else {
            eax += 0xADDR;
        }
    }
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = rdi;
    sub_14000217c(rcx);
    *(u64*)((rax + 0xADDR)) = r13;
    rcx = (int64)(*(u32*)((rax + 0xADDR)));
    rax = *(u64*)(r14);
    *(u64*)((rcx + rax)) = -0xADDR;
    rax = rbx;
    return;
}
