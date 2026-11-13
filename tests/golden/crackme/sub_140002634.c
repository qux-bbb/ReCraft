// L1 structure recovery (2 blocks)
void sub_140002634_l1(void) {
    rbx = rcx;
    var_40 = 0;
    var_50 = rcx;
    __vcrt_getptd(rcx);
    rdx = &var_50;
    ecx = *(u32*)(rbx);
    rax = *(u64*)((rax + 0xADDR));
    (*qword_ADDR)();
    var_40 = 0;
    eax = var_40;
    return;
}
