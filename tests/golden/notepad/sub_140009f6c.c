// L1 structure recovery (7 blocks)
void sub_140009f6c_l1(void) {
    var_30 = 0;
    rdi = (rcx + 0xADDR);
    rax = "@)";
    rsi = rcx;
    *(u64*)(rcx) = rax;
    *(u64*)(rdi) = 0;
    CoCreateFreeThreadedMarshaler(0, &var_30);
    if (eax >= 0 /* signed */) {
        rbx = var_30;
        rcx = *(u64*)(rdi);
        rax = *(u64*)(rbx);
        rbp = *(u64*)(rax);
        if (rcx != 0) {
            *(u64*)(rdi) = 0;
            rax = *(u64*)((*(u64*)(rcx) + 0xADDR));
            sub_140028010(rcx, *(u64*)(rcx));
        }
        r8 = rdi;
        rax = rbp;
        sub_140028010(rbx, 0xADDR);
    }
    rcx = var_30;
    if (rcx != 0) {
        var_30 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx);
    }
    rax = rsi;
    return;
}
