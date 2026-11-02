// L1 structure recovery (1 blocks)
void sub_1400012dc_l1(void) {
    eax = (uint32)(*(u8*)(rdx));
    r11 = rcx;
    eax <<= 0xADDR;
    r10 = r8;
    r8 = var_78;
    var_30 = eax;
    var_34 = (uint32)(*(u16*)((rdx + 1)));
    rdx += 0xADDR;
    *(u64*)(r8) = *(u64*)((rcx + 8));
    rax = *(u64*)((rcx + 8));
    ecx = (uint32)(*(u16*)(rax));
    *(u32*)((r8 + 8)) = ecx;
    rcx = off_14002e7f0;
    *(u64*)((r8 + 0xADDR)) = rdx;
    *(u32*)((r8 + 0xADDR)) = 2;
    eax = (uint32)(*(u16*)(rdx));
    *(u32*)((r8 + 0xADDR)) = eax;
    rax = off_14002ed7f;
    eax -= ecx;
    *(u32*)((r8 + 0xADDR)) = 1;
    var_78 = eax;
    eax = var_70;
    var_20 = eax;
    EventWriteTransfer(*(u64*)((r11 + 0xADDR)), &var_30, r10);
    return;
}
