// L1 structure recovery (14 blocks)
void sub_1400067b4_l1(void) {
    rdx = *(u64*)((rcx + 0xADDR));
    *(u64*)((rcx + 0xADDR)) = (rdx + 8);
    r9 = *(u64*)(rdx);
    if (r9 == 0) {
        rdx = "(null)";
        eax = 6;
        *(u64*)((rcx + 0xADDR)) = rdx;
        *(u8*)((rcx + 0xADDR)) = 0;
    } else {
        r10 = *(u64*)((r9 + 8));
        if (r10 == 0) {
            rdx = "(null)";
            eax = 6;
            *(u64*)((rcx + 0xADDR)) = rdx;
            *(u8*)((rcx + 0xADDR)) = 0;
            *(u32*)((rcx + 0xADDR)) = eax;
            al = 1;
            return;
        }
        edx = *(u32*)((rcx + 0xADDR));
        r8b = *(u8*)((rcx + 0xADDR));
        edx -= 2;
        if (edx == 0) {
            al = 0;
        } else {
            edx -= 1;
            if (edx == 0) {
                al = 1;
            } else if (edx != 9) {
                if (*(u32*)((rcx + 0xADDR)) != 0xADDR) {
                    r8b -= 0xADDR;
                    al = (uint8)(((r8b & 0xADDR) != 0));
                }
            }
        }
        *(u64*)((rcx + 0xADDR)) = r10;
        edx = (uint32)(*(u16*)(r9));
        eax = edx;
        if (al == 0) {
            *(u8*)((rcx + 0xADDR)) = 0;
            *(u32*)((rcx + 0xADDR)) = eax;
            al = 1;
            return;
        }
        *(u8*)((rcx + 0xADDR)) = 1;
        eax >>= 1;
    }
    *(u32*)((rcx + 0xADDR)) = eax;
    al = 1;
    return;
}
