// L1 structure recovery (5 blocks)
void sub_14001e800_l1(void) {
    PathFileExistsW(qword_ADDR);
    if (eax == 0) {
        sub_140026b60();
        if (eax == 0) {
            sub_14000c980((rax + 5));
        }
        return;
    }
    rcx = qword_ADDR;
    return sub_140012308();
}
