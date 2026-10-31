// L1 structure recovery (4 blocks)
void internal/abi.(*Type).Uncommon_l1(void) {
    if (rsp <= *(u64*)((r14 + 0xADDR))) return sub_140001634();
    var_8 = rbp;
    ecx = (uint32)(*(u8*)((rax + 0xADDR)));
    if ((cl & 1) == 0) return sub_14000162c();
    var_18 = rax;
    internal/abi.(*Type).Kind();
    ecx = (uint32)(al);
    rcx -= 0xADDR;
    if (rcx > 8) return sub_14000161d();
    rax = off_1400eace0;
    __asm("jmp qword ptr [rax+rcx*8]");
}
