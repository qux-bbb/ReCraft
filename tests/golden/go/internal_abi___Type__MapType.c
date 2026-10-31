// L1 structure recovery (5 blocks)
void internal/abi.(*Type).MapType_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_18 = rax;
        runtime.morestack_noctxt();
        rax = var_18;
        continue;
    }
    var_18 = rax;
    internal/abi.(*Type).Kind();
    if (al == 0xADDR) {
        rax = var_18;
        return;
    }
    eax = 0;
    return;
}
