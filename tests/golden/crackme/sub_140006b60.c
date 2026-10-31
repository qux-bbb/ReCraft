// L1 structure recovery (18 blocks)
void sub_140006b60_l1(void) {
    rdx = *(u64*)((rcx + 0xADDR));
    rbx = rcx;
    rax = (rdx + 8);
    *(u64*)((rcx + 0xADDR)) = rax;
    rdi = *(u64*)(rdx);
    sub_14000cb0c(rcx, *(u64*)((rcx + 0xADDR)));
    if (eax != 0) {
        ecx = *(u32*)((rbx + 0xADDR));
        if (ecx > 5 /* signed */) {
            ecx -= 6;
            if (ecx == 0) {
                *(u64*)(rdi) = (int64)(*(u32*)((rbx + 0xADDR)));
                *(u8*)((rbx + 0xADDR)) = 1;
                al = 1;
            } else {
                ecx -= 1;
                if (ecx == 0) {
                    *(u64*)(rdi) = (int64)(*(u32*)((rbx + 0xADDR)));
                    *(u8*)((rbx + 0xADDR)) = 1;
                    al = 1;
                    return;
                }
                ecx -= 2;
                if (ecx == 0) {
                    *(u64*)(rdi) = (int64)(*(u32*)((rbx + 0xADDR)));
                    *(u8*)((rbx + 0xADDR)) = 1;
                    al = 1;
                    return;
                }
                ecx -= 1;
                if (ecx == 0) {
                    *(u32*)(rdi) = *(u32*)((rbx + 0xADDR));
                    *(u8*)((rbx + 0xADDR)) = 1;
                    al = 1;
                    return;
                } else if (ecx != 1) {
                    rax = *(u64*)((rbx + 8));
                    r9d = 0;
                    r8d = 0;
                    edx = 0;
                    ecx = 0;
                    *(u8*)((rax + 0xADDR)) = 1;
                    *(u32*)((rax + 0xADDR)) = 0xADDR;
                    rax = *(u64*)((rbx + 8));
                    var_28 = rax;
                    var_20 = 0;
                    sub_140008bd0(0, 0, 0, 0, 0, *(u64*)((rbx + 8)));
                    al = 0;
                } else {
                    *(u64*)(rdi) = (int64)(*(u32*)((rbx + 0xADDR)));
                    *(u8*)((rbx + 0xADDR)) = 1;
                    al = 1;
                    return;
                }
            }
        } else if (ecx == 5) {
            *(u64*)(rdi) = (int64)(*(u32*)((rbx + 0xADDR)));
            *(u8*)((rbx + 0xADDR)) = 1;
            al = 1;
            return;
        } else if (ecx == 0) {
            *(u32*)(rdi) = *(u32*)((rbx + 0xADDR));
            *(u8*)((rbx + 0xADDR)) = 1;
            al = 1;
            return;
        }
        ecx -= 1;
        if (ecx == 0) {
            *(u8*)(rdi) = *(u8*)((rbx + 0xADDR));
            *(u8*)((rbx + 0xADDR)) = 1;
            al = 1;
            return;
        }
        ecx -= 1;
        if (ecx == 0) {
            eax = (uint32)(*(u16*)((rbx + 0xADDR)));
            *(u16*)(rdi) = ax;
            *(u8*)((rbx + 0xADDR)) = 1;
            al = 1;
            return;
        }
    } else {
        rax = *(u64*)((rbx + 8));
        r9d = 0;
        r8d = 0;
        edx = 0;
        ecx = 0;
        *(u8*)((rax + 0xADDR)) = 1;
        *(u32*)((rax + 0xADDR)) = 0xADDR;
        rax = *(u64*)((rbx + 8));
        var_28 = rax;
        var_20 = 0;
        sub_140008bd0(0, 0, 0, 0, 0, *(u64*)((rbx + 8)));
        al = 0;
        return;
    }
    return;
}
