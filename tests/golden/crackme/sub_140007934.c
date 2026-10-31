// L1 structure recovery (9 blocks)
void sub_140007934_l1(void) {
    edi = 0;
    if (qword_ADDR == rdi) {
        sub_14000e09c();
        sub_14000e5d8();
        rbx = rax;
        if (rax != 0) {
            sub_1400079a8(rbx);
            if (rax != 0) {
                qword_ADDR = rax;
                qword_ADDR = rax;
            } else {
                edi = -1;
            }
            ecx = 0;
            sub_140008ee0(0);
            sub_140008ee0(rbx);
            eax = edi;
        } else {
            ecx = 0;
            sub_140008ee0(0);
            eax = -1;
        }
    } else {
        eax = 0;
    }
    return;
}
