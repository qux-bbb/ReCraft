// L1 structure recovery (5 blocks)
void internal/abi.Kind.String_l1(void) {
    ecx = (uint32)(al);
    rdx = qword_ADDR;
    rsi = qword_ADDR;
    if (rdx > rcx /* signed */) {
        rcx <<= 4;
        rax = *(u64*)((rsi + rcx));
        return;
    }
    __asm("if (jbe short 0xADDR /* unmodelled flags */) goto 0xADDR");
    rax = *(u64*)(rsi);
    return;
}
