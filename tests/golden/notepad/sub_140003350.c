// L1 structure recovery (1 blocks)
void sub_140003350_l1(void) {
    r10d = edx;
    r11 = rcx;
    r9 = r8;
    sub_1400052b0(&var_60, 0xADDR);
    xmm0 = *(f64*)(rax);
    var_58 = *(u32*)((rax + 8));
    var_30 = &var_50;
    rax = var_a8;
    var_28 = rax;
    var_50 = xmm0;
    sub_14000312c(r11, r10d, r9);
}
