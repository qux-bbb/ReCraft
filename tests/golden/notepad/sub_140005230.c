// L1 structure recovery (3 blocks)
void sub_140005230_l1(void) {
    rax = qword_ADDR;
    ebx = r8d;
    rdi = rdx;
    esi = ecx;
    if (rax == 0) {
        GetModuleHandleW("ntdll.dll");
        qword_ADDR = rax;
    }
    var_30 = 0;
    var_28 = ebx;
    var_20 = rdi;
    FormatMessageW(0xADDR, rax, esi, 0xADDR, rdi, ebx, 0);
    return;
}
