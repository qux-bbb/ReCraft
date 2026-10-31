// L1 structure recovery (5 blocks)
void internal/abi.(*Type).Len_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_18 = rax;
        runtime.morestack_noctxt();
        rax = var_18;
        continue;
    }
    var_18 = rax;
    internal/abi.(*Type).Kind();
    if (al != 0xADDR) {
        eax = 0;
        return;
    }
    rax = *(u64*)((var_18 + 0xADDR));
    return;
}
