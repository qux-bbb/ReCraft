// L1 structure recovery (8 blocks)
void sub_1400018f4_l1(void) {
    GetModuleHandleW(0);
    if (rax == 0) {
        al = 0;
    } else {
        ecx = 0xADDR;
        if (*(u16*)(rax) != cx) {
            al = 0;
            return;
        }
        rcx = (int64)(*(u32*)((rax + 0xADDR)));
        if (*(u32*)((rcx + rax)) != 0xADDR) {
            al = 0;
            return;
        }
        edx = 0xADDR;
        if (*(u16*)(((rcx + rax) + 0xADDR)) != dx) {
            al = 0;
            return;
        } else if (*(u32*)(((rcx + rax) + 0xADDR)) <= 0xADDR) {
            al = 0;
            return;
        }
        al = (uint8)((*(u32*)(((rcx + rax) + 0xADDR)) != 0));
    }
    return;
}
