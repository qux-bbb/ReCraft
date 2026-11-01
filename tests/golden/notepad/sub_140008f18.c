// L1 structure recovery (8 blocks)
void sub_140008f18_l1(void) {
    if (rdx != 0) {
        rdi = rdx;
        rbx = (rcx + 4);
        do {
            rax = qword_ADDR;
            r8d = (uint32)(*(u16*)((rbx + 2)));
            edx = (uint32)(*(u16*)(rbx));
            ecx = *(u32*)((rbx - 4));
            if (rax != 0) {
                r9d = 0;
                sub_140028010();
            } else {
                rax = qword_ADDR;
            }
            rbx += 8;
            rdi -= 1;
        } while (rdi != 0);
    }
    return;
}
