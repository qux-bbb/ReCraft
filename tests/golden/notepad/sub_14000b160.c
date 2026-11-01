// L1 structure recovery (1 blocks)
void sub_14000b160_l1(void) {
    rect = 0;
    GetClientRect(qword_ADDR, &rect);
    edx = rect.bottom;
    ecx = rect.right;
    edx -= rect.top;
    ecx -= rect.left;
    sub_14000e848(rcx, rdx);
    return;
}
