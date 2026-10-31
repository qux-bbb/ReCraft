// L1 structure recovery (1 blocks)
void sub_140002c38_l1(void) {
    rbx = r8;
    sub_14000277c(rcx, rdx, r8, &var_40);
    rcx = *(u64*)(rax);
    rax = (int64)(*(u32*)((rbx + 0xADDR)));
    var_40 = rcx;
    eax = *(u32*)(((rax + rcx) + 4));
    return;
}
