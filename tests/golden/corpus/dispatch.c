// L1 structure recovery (1 blocks)
void dispatch_l1(void) {
    eax = (ecx & 3);
    rax = *(u64*)((off_140003008 + (rax * 8)));
    __asm("jmp rax");
}
