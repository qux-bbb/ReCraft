// L1 structure recovery (5 blocks)
void sub_140005ce0_l1(void) {
    rbp = *(u64*)((rcx + 8));
    rsi = rcx;
    rax = *(u64*)((rcx + 0xADDR));
    rdi = rdx;
    rcx = &var_40;
    rax -= 1;
    rbx = var_2c;
    r9b = 1;
    var_40 = rax;
    sub_140004f54(rbp, (r11 - 0xADDR), 0xADDR);
    *(u32*)(rdi) = eax;
    rax = *(u64*)((rsi + 8));
    if (*(u8*)((rax + 0xADDR)) == 0xADDR) {
        rax = var_40;
        if (rax >= *(u64*)((rsi + 0xADDR))) {
            *(u64*)((rsi + 0xADDR)) = rax;
            cl = 1;
        }
    }
    al = cl;
    *(u64*)((rbp + 0xADDR)) = rbx;
    return;
}
