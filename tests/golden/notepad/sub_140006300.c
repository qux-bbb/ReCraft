// L1 structure recovery (13 blocks)
void sub_140006300_l1(void) {
    eax = *(u32*)((rcx + 8));
    esi = 0;
    rdi = qword_ADDR;
    rbx = rcx;
    var_20 = eax;
    var_30 = *(u8*)(rcx);
    var_32 = ax;
    eax = (uint32)(*(u16*)((rcx + 0xADDR)));
    var_38 = ax;
    rax = *(u64*)((rcx + 0xADDR));
    var_24 = 0;
    var_31 = sil;
    var_34 = esi;
    var_3a = si;
    var_3c = esi;
    var_60 = rsi;
    if (rdi != 0) {
        ecx = 0;
        rax = rdi;
        sub_140028010(0, &var_20, &var_60);
    } else {
        rax = qword_ADDR;
        edi = esi;
        if (rax != 0) {
            GetProcAddress(rax, "WilFailureNotifyWatchers");
            rdi = rax;
        } else {
            GetModuleHandleW("kernelbase.dll");
            qword_ADDR = rax;
        }
        qword_ADDR = rdi;
    }
    ecx = (uint32)(var_64);
    eax = var_60;
    *(u32*)((rbx + 0xADDR)) = eax;
    switch (ecx) {
        case 0xADDR: {
            *(u32*)((rbx + 4)) = (*(u32*)((rbx + 4)) | 1);
            break;
        }
        case 0xADDR: {
            *(u32*)((rbx + 4)) = (*(u32*)((rbx + 4)) | 2);
            break;
        }
        case 0xADDR: {
            *(u32*)((rbx + 4)) = (*(u32*)((rbx + 4)) | 4);
            break;
        }
    }
    return;
}
