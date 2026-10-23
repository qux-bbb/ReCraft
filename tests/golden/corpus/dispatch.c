// L1 structure recovery (1 blocks)
void dispatch_l1(void) {
    eax = ecx;
    rdx = 0xADDR;
    eax = (eax & 3);
    rax = *(u64*)((rdx + (rax * 8)));
    __asm("jmp rax");
}
