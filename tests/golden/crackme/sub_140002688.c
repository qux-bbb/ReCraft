// L1 structure recovery (6 blocks)
void sub_140002688_l1(void) {
    edi = *(u32*)((rcx + 0xADDR));
    esi = edx;
    rbp = rcx;
    if (edi == 0) {
        eax = 0;
    } else {
        while (1) {
            ebx = (rdi - 1);
            edi = ebx;
            __vcrt_getptd(rcx);
            rdx = (rbx + (rbx * 4));
            rax = *(u64*)((rax + 0xADDR));
            rcx = (rax + (rdx * 4));
            rax = (int64)(*(u32*)((rbp + 0xADDR)));
            rax += rcx;
            if (esi <= *(u32*)((rax + 4)) /* signed */) {
                if (ebx != 0) continue;
            } else {
                if (esi <= *(u32*)((rax + 8)) /* signed */) break;
            }
            break;
        }
    }
    return;
}
