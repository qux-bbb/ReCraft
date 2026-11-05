// L1 structure recovery (22 blocks)
void sub_140023e28_l1(void) {
    esi = edx;
    rbx = rcx;
    if (rcx == 0) {
        eax = 0;
    } else {
        sub_140024a24(rcx, rdx);
        if (eax == 0) {
            eax = 0;
            return;
        } else if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) == 0) {
            if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) == 0) {
                r9d = 0xADDR;
                r8d = 0xADDR;
                if (esi != 0) {
                    r8d = 0xADDR;
                }
                sub_1400243c8(rcx);
                if (rax == 0) {
                    eax = 0;
                    return;
                }
                rcx = qword_ADDR;
                LoadResource();
                if (rax != 0) {
                    LockResource(rax);
                    rbp = rax;
                    if (rax == 0) {
                        eax = 0;
                        return;
                    }
                    edx = 0xADDR;
                    LocalAlloc((rdx + 0xADDR), 0xADDR);
                    rdi = rax;
                    if (rax == 0) {
                        eax = 0;
                        return;
                    }
                    if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) != 0) {
                        qword_ADDR = *(u64*)((rbx + 0xADDR));
                    }
                    rcx = qword_ADDR;
                    *(u64*)((rax + 8)) = rbx;
                    *(u32*)(rax) = esi;
                    var_20 = rdi;
                    CreateDialogIndirectParamW(qword_ADDR, rbp, *(u64*)((rbx + 8)), sub_140023f90, rdi);
                    rbx = rax;
                    if (rax == 0) {
                        qword_ADDR &= rax;
                        LocalFree(rdi);
                    }
                    rax = rbx;
                } else {
                    eax = 0;
                    return;
                }
            } else {
                rax = *(u64*)((rbx + 0xADDR));
                if (rax != 0) {
                    LockResource(rax);
                    rbp = rax;
                    if (rax == 0) {
                        eax = 0;
                        return;
                    }
                    edx = 0xADDR;
                    LocalAlloc((rdx + 0xADDR), 0xADDR);
                    rdi = rax;
                    if (rax == 0) {
                        eax = 0;
                        return;
                    }
                    if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) != 0) {
                        qword_ADDR = *(u64*)((rbx + 0xADDR));
                    }
                    rcx = qword_ADDR;
                    *(u64*)((rax + 8)) = rbx;
                    *(u32*)(rax) = esi;
                    var_20 = rdi;
                    CreateDialogIndirectParamW(qword_ADDR, rbp, *(u64*)((rbx + 8)), sub_140023f90, rdi);
                    rbx = rax;
                    if (rax == 0) {
                        qword_ADDR &= rax;
                        LocalFree(rdi);
                    }
                    rax = rbx;
                } else {
                    eax = 0;
                    return;
                }
                return;
            }
        } else {
            FindResourceW(*(u64*)((rbx + 0xADDR)), *(u64*)((rbx + 0xADDR)), 5);
            if (rax == 0) {
                eax = 0;
                return;
            }
            rcx = *(u64*)((rbx + 0xADDR));
        }
    }
    return;
}
