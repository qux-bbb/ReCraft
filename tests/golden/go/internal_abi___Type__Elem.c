// L1 structure recovery (14 blocks)
void internal/abi.(*Type).Elem_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_18 = rax;
        runtime.morestack_noctxt();
        rax = var_18;
        continue;
    }
    var_18 = rax;
    internal/abi.(*Type).Kind();
    if (al > 0xADDR) {
        if (al == 0xADDR) {
            rcx = var_18;
            rax = *(u64*)((rcx + 0xADDR));
            return;
        }
        if (al == 0xADDR) {
            rcx = var_18;
            rax = *(u64*)((rcx + 0xADDR));
            return;
        }
        if (al == 0xADDR) {
            rcx = var_18;
            rax = *(u64*)((rcx + 0xADDR));
            return;
        }
        eax = 0;
        return;
    }
    if (al == 0xADDR) {
        rcx = var_18;
        rax = *(u64*)((rcx + 0xADDR));
        return;
    }
    if (al != 0xADDR) {
        eax = 0;
        return;
    }
    rcx = var_18;
    rax = *(u64*)((rcx + 0xADDR));
    return;
}
