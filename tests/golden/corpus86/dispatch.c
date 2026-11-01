// L1 structure recovery (1 blocks)
void dispatch_l1(void) {
    eax = var_4;
    ecx = (eax & 3);
    var_4 = eax;
    eax = *(u32*)(((ecx * 4) + 0xADDR));
    __asm("jmp eax");
}
