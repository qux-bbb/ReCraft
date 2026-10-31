// L1 structure recovery (3 blocks)
void internal/abi.TypeOf_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_18 = rax;
        var_20 = rbx;
        runtime.morestack_noctxt();
        rax = var_18;
        rbx = var_20;
        continue;
    }
    var_18 = rax;
    var_20 = rbx;
    rax = var_18;
    internal/abi.NoEscape();
    return;
}
