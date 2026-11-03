// L1 structure recovery (5 blocks)
void internal/abi.(*Type).ExportedMethods_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        runtime.morestack_noctxt();
        continue;
    }
    internal/abi.(*Type).Uncommon();
    if (rax == 0) {
        eax = 0;
        rcx = 0;
        return;
    }
    internal/abi.(*UncommonType).ExportedMethods();
    return;
}
