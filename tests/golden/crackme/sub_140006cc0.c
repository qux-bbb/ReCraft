// L1 structure recovery (3 blocks)
void sub_140006cc0_l1(void) {
    rdi = rcx;
    sub_140004828(rcx);
    rdx = (rdi + 0xADDR);
    rsi = rax;
    *(u64*)(rdx) = *(u64*)((rax + 0xADDR));
    *(u64*)((rdi + 0xADDR)) = *(u64*)((rax + 0xADDR));
    sub_14000b4ac(rax, (rdi + 0xADDR), *(u64*)((rdi + 8)));
    sub_14000b518(rsi, (rdi + 0xADDR), *(u64*)((rdi + 8)));
    eax = *(u32*)((rsi + 0xADDR));
    if ((al & 2) == 0) {
        eax |= 2;
        *(u32*)((rsi + 0xADDR)) = eax;
        *(u8*)((rdi + 0xADDR)) = 2;
    }
    return;
}
