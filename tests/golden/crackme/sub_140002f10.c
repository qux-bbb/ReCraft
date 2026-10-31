// L1 structure recovery (12 blocks)
void sub_140002f10_l1(void) {
    rsi = r9;
    r14 = rcx;
    if (*(u32*)(r8) >= 0 /* signed */) {
        rdi = (int64)(*(u32*)((r8 + 8)));
        rdi += *(u64*)(rdx);
    } else {
        rdi = rdx;
    }
    sub_140002d1c(rcx, rdx, r8, r9);
    eax -= 1;
    if (eax == 0) {
        sub_140001ffc(*(u64*)((r14 + 0xADDR)), (rsi + 8));
        r14 = rax;
        rsi = (int64)(*(u32*)((rsi + 0xADDR)));
        if (esi != 0) {
            sub_140002b44(rcx);
            rbx = (rax + rsi);
        }
        r8 = r14;
        sub_140004258(rdi, rbx);
    } else if (eax == 1) {
        sub_140001ffc(*(u64*)((r14 + 0xADDR)), (rsi + 8));
        r14 = rax;
        rsi = (int64)(*(u32*)((rsi + 0xADDR)));
        if (esi != 0) {
            sub_140002b44(rcx);
            rbx = (rax + rsi);
        }
        r9d = 1;
        sub_140004264(rdi, rbx, r14);
    }
    return;
}
