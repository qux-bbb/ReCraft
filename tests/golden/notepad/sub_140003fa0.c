// L1 structure recovery (7 blocks)
void sub_140003fa0_l1(void) {
    rbx = rcx;
    rcx = *(u64*)((rcx + 8));
    if (rcx == 0) {
        rcx = *(u64*)(rbx);
        if (rcx == 0) {
            return;
        }
        CloseHandle(*(u64*)(rbx));
        if (eax == 0) {
            sub_1400091e4(var_28, 0xADDR);
        }
    }
    CloseHandle(rcx);
    if (eax == 0) {
        sub_1400091e4(var_28, 0xADDR);
    }
}
