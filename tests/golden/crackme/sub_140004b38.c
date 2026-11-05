// L1 structure recovery (11 blocks)
void sub_140004b38_l1(void) {
    eax = (*(u32*)((rcx + 0xADDR)) >> 0xADDR);
    if ((al & 1) != 0) {
        al = 1;
    } else {
        sub_140009a28(rcx);
        r9 = (int64)(eax);
        r10 = off_1400231d0;
        r8 = r9;
        eax = (r9 + 2);
        if (eax <= 1) {
            rdx = r10;
        } else {
            rax = r9;
            rdx = (r9 >> 6);
            eax &= 0xADDR;
            rcx = (rax + (rax * 8));
            rax = *(u64*)((off_140023f50 + (rdx * 8)));
            rdx = (rax + (rcx * 8));
        }
        if (*(u8*)((rdx + 0xADDR)) != 0) {
            sub_140008e30();
            *(u32*)(rax) = 0xADDR;
            sub_140008c88();
            al = 0;
        } else {
            eax = (r9 + 2);
            if (eax > 1) {
                rax = (r8 >> 6);
                r8d &= 0xADDR;
                rax = *(u64*)((off_140023f50 + (rax * 8)));
                rcx = (r8 + (r8 * 8));
                r10 = (rax + (rcx * 8));
            }
            if ((*(u8*)((r10 + 0xADDR)) & 1) == 0) {
                al = 1;
                return;
            }
            sub_140008e30();
            *(u32*)(rax) = 0xADDR;
            sub_140008c88();
            al = 0;
            return;
        }
    }
    return;
}
