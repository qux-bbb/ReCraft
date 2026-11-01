// L1 structure recovery (6 blocks)
void sub_14000f170_l1(void) {
    if (edx == 0xADDR) {
        if (*(u32*)((r9 + 0xADDR)) == 0xADDR) {
            var_20 = 0;
            edx = 0xADDR;
            SendDlgItemMessageW(rcx, 0xADDR, (rdx + 0xADDR), 0, 0);
            dword_ADDR = eax;
        }
    } else if (edx == 0xADDR) {
        r8 = qword_ADDR;
        sub_14000cec0(rcx, 1);
    }
    eax = 0;
    return;
}
