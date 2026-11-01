// L1 structure recovery (1 blocks)
void internal/abi.(*MapType).NeedKeyUpdate_l1(void) {
    eax = (*(u32*)((rax + 0xADDR)) & 1);
    return;
}
