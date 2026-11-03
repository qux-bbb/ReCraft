// L1 structure recovery (13 blocks)
void sub_1400082b0_l1(void) {
    rax = (rdx - 1);
    r11 = r8;
    r9 = rdx;
    rbx = rcx;
    if (rax > 0xADDR) {
    } else {
        r10 = 0xADDR;
        rax = rcx;
        edi = 0;
        while (*(u16*)(rax) != di) {
            rax += 2;
            r10 -= 1;
            if (r10 == 0) break;
        }
        rax = r10;
        rcx = r9;
        rax = r10;
        edx = ((rax != 0) ? -1 : 0);
        rcx -= r10;
        edx = (~(edx) & 0xADDR);
        rax = -(rax);
        r8 = ((rax != 0) ? rcx : 0);
        if (r10 != 0) {
            rcx = r9;
            rdx = (rbx + (r8 * 2));
            rcx -= r8;
            if (rcx != 0) {
                r8 -= r9;
                rax = (r8 + 0xADDR);
                rax += rcx;
                r11 -= rdx;
                while (rax != 0) {
                    r8d = (uint32)(*(u16*)((r11 + rdx)));
                    if (r8w == 0) break;
                    *(u16*)(rdx) = r8w;
                    rax -= 1;
                    rdx += 2;
                    rcx -= 1;
                    if (rcx == 0) break;
                }
            }
            rax = (rdx - 2);
            rax = ((rcx != 0) ? rdx : rax);
            rcx = -(rcx);
            edx = ((rcx != 0) ? -1 : 0);
            edx = (~(edx) & 0xADDR);
            *(u16*)(rax) = di;
        }
    }
    eax = edx;
    return;
}
