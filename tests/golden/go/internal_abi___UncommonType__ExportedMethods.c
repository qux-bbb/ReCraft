// L1 structure recovery (5 blocks)
void internal/abi.(*UncommonType).ExportedMethods_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_30 = rax;
        runtime.morestack_noctxt();
        rax = var_30;
        continue;
    }
    if (*(u16*)((rax + 6)) != 0) {
        var_30 = rax;
        internal/abi.addChecked(off_1400dc21b);
        ecx = (uint32)(*(u16*)((var_30 + 6)));
        return;
    }
    eax = 0;
    rcx = 0;
    return;
}
