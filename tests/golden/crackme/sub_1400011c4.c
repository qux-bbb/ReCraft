// L1 structure recovery (9 blocks)
void sub_1400011c4_l1(void) {
    sub_140007504(1);
    sub_140001734();
    sub_140008050(eax);
    sub_140001728();
    ebx = eax;
    sub_14000821c();
    *(u32*)(rax) = ebx;
    sub_1400014b0(1);
    if (al == 0) {
        __scrt_fastfail(7);
    }
    _RTC_Initialize();
    atexit(sub_1400019f8);
    sub_14000172c();
    sub_1400077b4(eax);
    if (eax != 0) {
        __scrt_fastfail(7);
        return;
    }
    sub_14000173c();
    sub_140001778();
    if (eax != 0) {
        sub_140007570(sub_140001728);
    }
    sub_140001750();
    sub_140001750();
    sub_140001728();
    sub_140008174(eax);
    sub_14000174c();
    if (al != 0) {
        sub_140007ce8();
    }
    sub_140001728();
    sub_1400018ec();
    if (eax != 0) {
        __scrt_fastfail(7);
        return;
    }
    return;
}
