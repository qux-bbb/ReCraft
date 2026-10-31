// L1 structure recovery (3 blocks)
void sub_140002c70_l1(void) {
    edi = r9d;
    rbx = r8;
    sub_14000277c(rcx, rdx, r8, &var_40);
    rcx = *(u64*)(rax);
    rax = (int64)(*(u32*)((rbx + 0xADDR)));
    var_40 = rcx;
    if (edi > *(u32*)(((rax + rcx) + 4)) /* signed */) {
        *(u32*)(((rax + rcx) + 4)) = edi;
    }
    return;
}
