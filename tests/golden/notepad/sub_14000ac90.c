// L1 structure recovery (3 blocks)
void sub_14000ac90_l1(void) {
    *(u32*)((rcx + 0xADDR)) = r8d;
    rcx = *(u64*)((rcx + 0xADDR));
    SetEvent(*(u64*)((rcx + 0xADDR)));
    if (eax == 0) {
        sub_1400091e4(var_28, 0xADDR);
    }
    eax = 0;
    return;
}
