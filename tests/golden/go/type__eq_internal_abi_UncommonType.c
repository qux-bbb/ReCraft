// L1 structure recovery (5 blocks)
void type:.eq.internal/abi.UncommonType_l1(void) {
    ecx = *(u32*)(rax);
    if (*(u32*)(rbx) != ecx) {
        eax = 0;
        return;
    }
    ecx = (uint32)(*(u16*)((rax + 4)));
    if (*(u16*)((rbx + 4)) != cx) {
        eax = 0;
        return;
    }
    ecx = (uint32)(*(u16*)((rax + 6)));
    if (*(u16*)((rbx + 6)) != cx) {
        eax = 0;
        return;
    }
    ecx = *(u32*)((rax + 8));
    al = (uint8)((*(u32*)((rbx + 8)) == ecx));
    return;
}
