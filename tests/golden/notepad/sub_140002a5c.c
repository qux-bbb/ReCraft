// L1 structure recovery (1 blocks)
void sub_140002a5c_l1(void) {
    rbx = rdx;
    rdi = rcx;
    sub_1400016f4();
    var_28 = rbx;
    r9 = rdi;
    var_20 = 0;
    r8d = 0;
    edx = 0;
    rcx = *(u64*)(rax);
    rcx |= 2;
    sub_1400028bc(rcx, 0, 0, rdi);
    eax = ((eax < 0 /* signed */) ? -1 : eax);
    return;
}
