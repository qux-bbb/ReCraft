// L1 structure recovery (5 blocks)
void internal/abi.(*FuncType).InSlice_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        var_30 = rax;
        runtime.morestack_noctxt();
        rax = var_30;
        continue;
    }
    ebx = 0xADDR;
    edx = 0xADDR;
    rbx = (((dl & 1) != 0) ? rdx : rbx);
    if (*(u16*)((rax + 0xADDR)) != 0) {
        var_30 = rax;
        internal/abi.addChecked(off_1400dc677);
        rdx = var_30;
        ecx = (uint32)(*(u16*)((rdx + 0xADDR)));
        return;
    }
    eax = 0;
    ebx = 0;
    rcx = rbx;
    return;
}
