// L1 structure recovery (4 blocks)
void sub_140004050_l1(void) {
    rbp = rcx;
    rdi = r8;
    rsi = rdx;
    sub_140002cac(r8);
    ebx = eax;
    sub_14000277c(rbp, rsi, rdi, &var_48);
    sub_140002c38(rbp, rsi, rdi);
    if (ebx <= eax /* signed */) {
        sub_140002c38(rbp, rsi, rdi);
        ebx = eax;
    } else {
        r8d = ebx;
        rdx = rdi;
        sub_140002c64(&var_48);
        sub_140002c70(rbp, rsi, rdi, ebx);
    }
    eax = ebx;
    return;
}
