// L1 structure recovery (10 blocks)
void sub_140005a24_l1(void) {
    rax = *(u64*)((rcx + 0xADDR));
    r9 = rcx;
    rcx += 0xADDR;
    bl = r8b;
    r11 = rdx;
    if (rax != 0) {
        rcx = rax;
        rax = (*(u64*)((r9 + 0xADDR)) >> 1);
    } else {
        eax = 0xADDR;
    }
    r8 = (rcx - 1);
    r8 += rax;
    *(u64*)((r9 + 0xADDR)) = r8;
    r10 = r8;
    while (1) {
        eax = *(u32*)((r9 + 0xADDR));
        if (eax > 0 /* signed */) {
            eax -= 1;
            *(u32*)((r9 + 0xADDR)) = eax;
            rax = 0xADDR;
            __asm("mul r11");
            rdx >>= 3;
            al = (dl << 2);
            ecx = (rax + rdx);
            cl += cl;
            r11b -= cl;
            r11b += 0xADDR;
            al = r11b;
            cl = r11b;
            r11 = rdx;
            if (al > 0xADDR /* signed */) {
                al = (((bl ^ 1) << 5) + 7);
                al += cl;
            }
            *(u8*)(r10) = al;
            *(u64*)((r9 + 0xADDR)) -= 1;
            r10 = *(u64*)((r9 + 0xADDR));
            continue;
        }
        if (r11 == 0) break;
    }
    r8d -= *(u32*)((r9 + 0xADDR));
    rax = (r10 + 1);
    *(u32*)((r9 + 0xADDR)) = r8d;
    *(u64*)((r9 + 0xADDR)) = rax;
    return;
}
