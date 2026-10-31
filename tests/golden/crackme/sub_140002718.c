// L1 structure recovery (4 blocks)
void sub_140002718_l1(void) {
    rbx = r8;
    rdi = rdx;
    sub_14000277c(rcx, rdx, r8, &var_48);
    rdx = rdi;
    rsi = rax;
    sub_140002cac(rbx);
    sub_140002688(rbx, eax);
    if (rax != 0) {
        r9d = *(u32*)((rax + 4));
    } else {
        r9d = -1;
    }
    sub_140003ec4(rsi, rdi, rbx, r9);
    return;
}
