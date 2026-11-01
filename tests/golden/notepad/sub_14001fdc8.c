// L1 structure recovery (5 blocks)
void sub_14001fdc8_l1(void) {
    al = byte_ADDR;
    r9d = edx;
    rbx = var_70;
    al = -(al);
    rdi = rcx;
    edx = ((al != 0) ? -1 : 0);
    rax = off_140036540;
    var_28 = -1;
    r8 = rcx;
    edx += 0xADDR;
    FindNLSString(0xADDR, (edx + 0xADDR), rcx, edx, off_140036540, (var_28 | -1), var_70);
    if (eax == -1) {
        eax = 0;
    } else if (*(u32*)(rbx) < 1 /* signed */) {
        eax = 0;
        return;
    }
    rax = (int64)(eax);
    rax = (rdi + (rax * 2));
    return;
}
