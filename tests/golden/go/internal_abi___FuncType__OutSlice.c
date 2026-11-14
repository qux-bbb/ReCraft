// L1 structure recovery (7 blocks)
void internal/abi.(*FuncType).OutSlice_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        var_38 = rax;
        internal/abi.(*FuncType).NumOut();
        if (ax == 0) break;
        var_8 = rax;
        rax = var_38;
        ebx = 0xADDR;
        edx = 0xADDR;
        rbx = (((dl & 1) != 0) ? rdx : rbx);
        internal/abi.addChecked(off_1400dc227, 0xADDR);
        rdx = var_38;
        edx = (uint32)(*(u16*)((rdx + 0xADDR)));
        rsi = var_8;
        esi += edx;
        ecx = (uint32)(si);
        if (rdx > rcx) {
            runtime.panicBounds(rcx, rdx, r8, r9);
        } else {
            return;
        }
    }
    eax = 0;
    rcx = 0;
    return;
}
