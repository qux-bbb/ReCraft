// L1 structure recovery (6 blocks)
void sub_140008bb8_l1(void) {
    rax = qword_ADDR;
    rbx = rcx;
    if (rax != 0) {
        rcx = rbx;
        sub_140028010(rbx);
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "RtlUnregisterFeatureConfigurationChangeNotification");
        qword_ADDR = rax;
        if (rax != 0) {
            rcx = rbx;
            sub_140028010(rbx);
            return;
        }
    }
    return;
}
