// L1 structure recovery (13 blocks)
void sub_140001f54_l1(void) {
    if (rcx != 0) {
        var_58 = dl;
        if (*(u32*)(rcx) == 0xADDR && *(u32*)((rcx + 0xADDR)) == 4) {
            eax = *(u32*)((rcx + 0xADDR));
            if (eax == 0xADDR) {
                rax = *(u64*)((rcx + 0xADDR));
                if (rax != 0) {
                    rdx = (int64)(*(u32*)((rax + 4)));
                    if (edx == 0) {
                        if ((*(u8*)(rax) & 0xADDR) != 0) {
                            rax = *(u64*)((rcx + 0xADDR));
                            rcx = *(u64*)(rax);
                            if (rcx != 0) {
                                rax = *(u64*)(rcx);
                                rax = *(u64*)((rax + 0xADDR));
                                (*qword_ADDR)();
                            }
                        }
                    } else {
                        rdx += *(u64*)((rcx + 0xADDR));
                        rcx = *(u64*)((rcx + 0xADDR));
                        sub_140001fc8(rcx, rdx);
                    }
                }
            } else {
                eax += 0xADDR;
            }
        }
    }
    return;
}
