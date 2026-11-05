// L1 structure recovery (15 blocks)
void sub_1400081ac_l1(void) {
    edi = *(u32*)((rcx + 0xADDR));
    ebp = 0;
    rsi = rdx;
    rbx = rcx;
    r14d = (rbp + 0xADDR);
    if (*(u64*)((rcx + 0xADDR)) == rbp && edi != 0) {
        sub_140006634((rbp + 8), 0xADDR);
        *(u64*)((rbx + 0xADDR)) = rax;
        if (rax != 0) {
            *(u32*)((rbx + 0xADDR)) = 5;
            rcx = (rax + 0xADDR);
            while (rax != rcx) {
                *(u16*)(rax) = r14w;
                rax += r14;
            }
        }
    }
    r8 = *(u64*)((rbx + 0xADDR));
    if (r8 != 0) {
        if (edi == 0) {
            eax = (uint32)(*(u16*)((rbx + 0xADDR)));
            ecx = (uint32)(*(u16*)((rbx + 0xADDR)));
            eax += 1;
            __asm("div ecx");
            eax = (uint32)(dx);
            *(u16*)((rbx + 0xADDR)) = 0xADDR;
            rcx = (rax + (rax * 4));
            rax = *(u64*)((rbx + 8));
            rcx <<= 4;
            rcx += r8;
            *(u32*)(rax) += 1;
            r8d = ((*(u32*)(rax) - r8d) + 1);
            sub_140007e20(rcx, rsi, r8);
        } else {
            eax = (uint32)(*(u16*)((rbx + 0xADDR)));
            rcx = ((rax + (rax * 4)) << 4);
            rcx += r8;
            if (r8 == rcx) {
                eax = (uint32)(*(u16*)((rbx + 0xADDR)));
                ecx = (uint32)(*(u16*)((rbx + 0xADDR)));
                eax += 1;
                __asm("div ecx");
                eax = (uint32)(dx);
                *(u16*)((rbx + 0xADDR)) = 0xADDR;
                rcx = (rax + (rax * 4));
                rax = *(u64*)((rbx + 8));
                rcx <<= 4;
                rcx += r8;
                *(u32*)(rax) += 1;
                r8d = ((*(u32*)(rax) - r8d) + 1);
                sub_140007e20(rcx, rsi, r8);
                return;
            }
            r9d = *(u32*)((rbx + 0xADDR));
            rdx = (r8 + 8);
            while (1) {
                if (*(u32*)((rdx - 4)) <= r9d) {
                    rdx += r14;
                    rax = (rdx - 8);
                    if (rax != rcx) continue;
                } else {
                    eax = *(u32*)((rsi + 8));
                    if (*(u32*)(rdx) == eax) break;
                }
            }
        }
    }
    return;
}
