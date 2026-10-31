// L1 structure recovery (12 blocks)
void sub_140006c18_l1(void) {
    rdx = *(u64*)((rcx + 0xADDR));
    rbx = rcx;
    *(u64*)((rcx + 0xADDR)) = (rdx + 8);
    eax = 0xADDR;
    rcx = *(u64*)(rdx);
    r8d = *(u32*)((rbx + 0xADDR));
    edx = *(u32*)((rbx + 0xADDR));
    r9b = *(u8*)((rbx + 0xADDR));
    r8d = ((r8d == -1) ? eax : r8d);
    *(u64*)((rbx + 0xADDR)) = rcx;
    edx -= 2;
    if (edx == 0) {
        if (rcx == 0) {
            *(u64*)((rbx + 0xADDR)) = "(null)";
        }
        sub_14000b0e0(rcx, (int64)(r8d));
    } else {
        edx -= 1;
        if (edx == 0) {
            if (rcx == 0) {
                *(u64*)((rbx + 0xADDR)) = "(null)";
            }
            *(u8*)((rbx + 0xADDR)) = 1;
            sub_14000b270(rcx, (int64)(r8d));
        } else if (edx != 9) {
            if (*(u32*)((rbx + 0xADDR)) != 0xADDR) {
                r9b -= 0xADDR;
                al = (uint8)(((r9b & 0xADDR) != 0));
            }
        }
    }
    *(u32*)((rbx + 0xADDR)) = eax;
    al = 1;
    return;
}
