// L1 structure recovery (3 blocks)
void internal/abi.(*StructField).Embedded_l1(void) {
    while (rsp <= *(u64*)((r14 + 0xADDR))) {
        runtime.morestack_noctxt();
        continue;
    }
    rax = *(u64*)(rax);
    internal/abi.Name.IsEmbedded();
    return;
}
