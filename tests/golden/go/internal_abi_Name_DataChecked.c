// L1 structure recovery (3 blocks)
void internal/abi.Name.DataChecked_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_40 = rcx;
        runtime.morestack_noctxt();
        rcx = var_40;
        continue;
    }
    var_40 = rcx;
    internal/abi.addChecked();
    return;
}
