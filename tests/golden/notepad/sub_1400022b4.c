// L1 structure recovery (8 blocks)
void sub_1400022b4_l1(void) {
    GetModuleHandleW(0);
    if (rax == 0) {
        al = 0;
    } else {
        if (*(u16*)(rax) != 0xADDR) {
            al = 0;
            return;
        }
        rcx = (int64)(*(u32*)((rax + 0xADDR)));
        rcx += rax;
        if (*(u32*)(rcx) != 0xADDR) {
            al = 0;
            return;
        }
        if (*(u16*)((rcx + 0xADDR)) != 0xADDR) {
            al = 0;
            return;
        } else if (*(u32*)((rcx + 0xADDR)) <= 0xADDR) {
            al = 0;
            return;
        }
        al = (uint8)((*(u32*)((rcx + 0xADDR)) != 0));
    }
    return;
}
