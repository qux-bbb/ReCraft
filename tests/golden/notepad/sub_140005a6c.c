// L1 structure recovery (15 blocks)
void sub_140005a6c_l1(void) {
    rbx = qword_ADDR;
    edi = 0;
    if (rbx != 0) {
        if (*(u64*)((rbx + 8)) == rdi) {
            var_38 &= rdi;
            sub_140004264(*(u64*)(rbx), &var_38);
            if (eax >= 0 /* signed */ && *(u64*)((rbx + 8)) == rdi) {
                *(u64*)((rbx + 8)) = var_38;
            }
        }
        rax = *(u64*)((rbx + 8));
        rcx = (rax + 0xADDR);
        rax = -(rax);
        rbx = ((rax != 0) ? rcx : 0);
        if (rbx != 0) {
            GetCurrentThreadId();
            r8d = eax;
            r9d = eax;
            rax = 0xADDR;
            __asm("mul r9");
            rdx >>= 3;
            rcx = (rdx + (rdx * 4));
            rcx += rcx;
            r8 -= rcx;
            rcx = *(u64*)(((rbx + (r8 * 8)) + 8));
            while (rcx != 0) {
                if (*(u32*)(rcx) != r9d) {
                    rcx = *(u64*)((rcx + 8));
                }
            }
        }
    }
    rax = rdi;
    return;
}
