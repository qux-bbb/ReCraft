// L1 structure recovery (3 blocks)
void dispatch_l1(void) {
    __x86.get_pc_thunk.dx();
    edx += 0xADDR;
    eax = var_4;
    if (eax > 3) {
        eax = 0xADDR;
        return;
    }
    var_4 = eax;
    __asm("jmp dword ptr [edx+eax*4-0xADDR]");
}
