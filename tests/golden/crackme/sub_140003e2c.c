// L1 structure recovery (14 blocks)
void sub_140003e2c_l1(void) {
    r9 = *(u64*)(rcx);
    rbx = r8;
    ecx = 0xADDR;
    *(u32*)(r8) = 0;
    if (*(u32*)(r9) != 0xADDR) {
        eax = 0;
    } else {
        r8d = 0xADDR;
        if (*(u32*)((r9 + 0xADDR)) == 4) {
            eax = *(u32*)((r9 + 0xADDR));
            if (eax == 0xADDR) {
                rax = *(u64*)((rdx + 0xADDR));
                if (*(u64*)((r9 + 0xADDR)) == rax) {
                    *(u32*)(rbx) = 1;
                }
            } else {
                eax += 0xADDR;
            }
        }
        if (*(u32*)(r9) != ecx) {
            eax = 0;
            return;
        } else if (*(u32*)((r9 + 0xADDR)) != 4) {
            eax = 0;
            return;
        }
        ecx = *(u32*)((r9 + 0xADDR));
        if (ecx == r8d) {
            if (*(u64*)((r9 + 0xADDR)) != 0) {
                eax = 0;
                return;
            }
            sub_14000217c(rcx);
            *(u32*)((rax + 0xADDR)) = 1;
            eax = 1;
            *(u32*)(rbx) = 1;
        } else {
            ecx += 0xADDR;
            if (ecx > 1) {
                eax = 0;
                return;
            }
        }
    }
    return;
}
