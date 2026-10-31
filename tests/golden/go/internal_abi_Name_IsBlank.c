// L1 structure recovery (8 blocks)
void internal/abi.Name.IsBlank_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        runtime.morestack_noctxt();
        continue;
    }
    if (rax == 0) {
        eax = 0;
        return;
    }
    ebx = 1;
    internal/abi.Name.ReadVarint();
    if (rbx == 1) {
        ebx = 2;
        internal/abi.Name.Data();
        cl = (uint8)((*(u8*)(rax) == 0xADDR));
    } else {
        ecx = 0;
    }
    eax = ecx;
    return;
}
