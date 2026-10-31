// L1 structure recovery (4 blocks)
void sub_14000231c_l1(void) {
    ebx = dword_ADDR;
    while (ebx != 0) {
        rax = off_140023c30;
        ebx -= 1;
        rcx = (rbx + (rbx * 4));
        rcx = (rax + (rcx * 8));
        DeleteCriticalSection((rax + (rcx * 8)));
        dword_ADDR = (dword_ADDR - 1);
    }
    al = 1;
    return;
}
