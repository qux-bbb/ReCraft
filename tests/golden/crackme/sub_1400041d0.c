// L1 structure recovery (10 blocks)
void sub_1400041d0_l1(void) {
    ebp = 0;
    rdi = rcx;
    if (*(u32*)(rcx) <= ebp /* signed */) {
        al = 0;
    } else {
        esi = 0;
        while (1) {
            rbx = (int64)(*(u32*)((rdi + 4)));
            sub_140002b30(rcx);
            rax += rsi;
            if (*(u32*)(((rax + rbx) + 4)) == 0) {
                eax = 0;
            } else {
                rbx = (int64)(*(u32*)((rdi + 4)));
                sub_140002b30(rcx);
                rax += rsi;
                rbx = (int64)(*(u32*)(((rax + rbx) + 4)));
                sub_140002b30(rcx);
                rax += rbx;
            }
            sub_140001f3c((rax + 8), off_140023ac8);
            if (eax == 0) break;
            ebp += 1;
            rsi += 0xADDR;
            if (ebp < *(u32*)(rdi) /* signed */) continue;
        }
        al = 1;
    }
    return;
}
