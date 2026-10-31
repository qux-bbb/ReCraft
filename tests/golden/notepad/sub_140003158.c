// L1 structure recovery (3 blocks)
void sub_140003158_l1(void) {
    rsi = var_a8;
    var_28 = rsi;
    ebx = edx;
    rdi = rcx;
    sub_14000576c(rcx, rdx, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\resource.h");
    if (eax > 0 /* signed */) {
        eax = (uint32)(ax);
        eax |= 0xADDR;
    }
    sub_1400052b0(&var_60, eax);
    xmm0 = *(f64*)(rax);
    eax = *(u32*)((rax + 8));
    var_58 = eax;
    rax = &var_50;
    var_30 = rax;
    var_28 = rsi;
    var_50 = xmm0;
    sub_14000312c(rdi, ebx, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\resource.h");
}
