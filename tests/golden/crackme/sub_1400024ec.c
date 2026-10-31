// L1 structure recovery (3 blocks)
void sub_1400024ec_l1(void) {
    ebx = ecx;
    sub_140002354(1, "FlsFree", off_140019390, "FlsFree");
    ecx = ebx;
    if (rax == 0) {
        TlsFree();
        return;
    }
    (*qword_ADDR)();
    return;
}
