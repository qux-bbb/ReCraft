// L1 structure recovery (5 blocks)
void internal/abi.(*Kind).String_l1(void) {
    while (1) {
        if (rsp <= *(u64*)((r14 + 0xADDR))) {
            runtime.morestack_noctxt();
            continue;
        }
        if (rax != 0) break;
        runtime.panicwrap(rcx, rdx, r8);
    }
    eax = (uint32)(*(u8*)(rax));
    internal/abi.Kind.String(rcx, rdx);
    return;
}
