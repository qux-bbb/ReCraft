// L1 structure recovery (5 blocks)
void sub_140027338_l1(void) {
    r11d = *(u32*)(r8);
    rbx = rdx;
    r11d &= -0xADDR;
    r9 = rcx;
    r10 = rcx;
    if ((*(u8*)(r8) & 4) != 0) {
        eax = *(u32*)((r8 + 8));
        r10 = (int64)(*(u32*)((r8 + 4)));
        eax = -(eax);
        r10 += rcx;
        rcx = (int64)(eax);
        r10 &= rcx;
    }
    rax = (int64)(r11d);
    rdx = *(u64*)((rax + r10));
    rax = *(u64*)((rbx + 0xADDR));
    ecx = *(u32*)((rax + 8));
    rax = *(u64*)((rbx + 8));
    if ((*(u8*)(((rcx + rax) + 3)) & 0xADDR) != 0) {
        eax = (uint32)(*(u8*)(((rcx + rax) + 3)));
        eax &= -0xADDR;
        r9 += rax;
    }
    r9 ^= rdx;
    rcx = r9;
    return sub_1400019f0();
}
