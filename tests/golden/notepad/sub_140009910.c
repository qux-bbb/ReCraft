// L1 structure recovery (7 blocks)
void sub_140009910_l1(void) {
    rbx = qword_ADDR;
    dil = r8b;
    esi = edx;
    ebp = ecx;
    if (rbx != 0) {
        r8b = dil;
        edx = esi;
        ecx = ebp;
        rax = rbx;
        sub_140028010();
    } else {
        rax = qword_ADDR;
        if (rax != 0) {
            GetProcAddress(rax, "LogStagedFeatureUsage");
            rbx = rax;
        } else {
            GetModuleHandleW("kernelbase.dll");
            qword_ADDR = rax;
        }
        qword_ADDR = rbx;
        if (rbx != 0) {
            r8b = dil;
            edx = esi;
            ecx = ebp;
            rax = rbx;
            sub_140028010();
            return;
        }
    }
    return;
}
