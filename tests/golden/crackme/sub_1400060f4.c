// L1 structure recovery (41 blocks)
void sub_1400060f4_l1(void) {
    al = *(u8*)((rcx + 0xADDR));
    if (al != 0xADDR) {
        if (al != 0xADDR) {
            if (*(u32*)((rcx + 0xADDR)) != 0) {
                rax = *(u64*)((rcx + 8));
                r9d = 0;
                r8d = 0;
                edx = 0;
                *(u8*)((rax + 0xADDR)) = 1;
                *(u32*)((rax + 0xADDR)) = 0xADDR;
                rax = *(u64*)((rcx + 8));
                ecx = 0;
                var_28 = rax;
                var_20 = 0;
                sub_140008bd0(0, 0, 0, 0, 0, *(u64*)((rcx + 8)));
                al = 0;
            } else {
                if (al == 0xADDR) {
                    r8 = *(u64*)((rcx + 0xADDR));
                    al = *(u8*)(r8);
                    if (al != 0xADDR) {
                        if (al != 0xADDR) {
                            al -= 0xADDR;
                            if (al <= 0xADDR && ((0xADDR >> rax) & 1) != 0) {
                                *(u32*)((rcx + 0xADDR)) = 9;
                            }
                        } else if (*(u8*)((r8 + 1)) == 0xADDR) {
                            rax = (r8 + 2);
                            *(u32*)((rcx + 0xADDR)) = 0xADDR;
                            *(u64*)((rcx + 0xADDR)) = rax;
                        }
                    } else if (*(u8*)((r8 + 1)) == 0xADDR) {
                        rax = (r8 + 2);
                        *(u32*)((rcx + 0xADDR)) = 0xADDR;
                        *(u64*)((rcx + 0xADDR)) = rax;
                    }
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 8;
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 0xADDR;
                } else if (al == 0xADDR) {
                    rax = *(u64*)((rcx + 0xADDR));
                    if (*(u8*)(rax) != 0xADDR) {
                        eax = 2;
                    } else {
                        rax += 1;
                        *(u64*)((rcx + 0xADDR)) = rax;
                        eax = 1;
                    }
                    *(u32*)((rcx + 0xADDR)) = eax;
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 5;
                } else if (al == 0xADDR) {
                    rax = *(u64*)((rcx + 0xADDR));
                    if (*(u8*)(rax) != 0xADDR) {
                        *(u32*)((rcx + 0xADDR)) = 3;
                        al = 1;
                        return;
                    }
                    rax += 1;
                    *(u64*)((rcx + 0xADDR)) = rax;
                    *(u32*)((rcx + 0xADDR)) = 4;
                    al = 1;
                    return;
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 7;
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 0xADDR;
                } else if (al == 0xADDR) {
                    *(u32*)((rcx + 0xADDR)) = 6;
                }
                al = 1;
            }
        } else if ((*(u8*)(rcx) & 8) != 0) {
            al = 1;
            return;
        }
        *(u8*)((rcx + 0xADDR)) = 8;
        return;
    }
    if ((*(u8*)(rcx) & 8) != 0) {
        al = 1;
        return;
    }
    *(u8*)((rcx + 0xADDR)) = 7;
    return sub_14000627c();
    eax = 3;
}
