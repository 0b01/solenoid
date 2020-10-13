define dso_local void @upow(i256* noalias nocapture sret align 8 %0, i256* nocapture readonly byval(i256) align 8 %1, i256* nocapture readonly byval(i256) align 8 %2) {
  %4 = load i256, i256* %1, align 8
  %5 = load i256, i256* %2, align 8
  %6 = and i256 %5, 1
  %7 = icmp eq i256 %6, 0
  %8 = select i1 %7, i256 1, i256 %4
  %9 = ashr i256 %5, 1
  %10 = icmp eq i256 %9, 0
  br i1 %10, label %22, label %11

11:                                               ; preds = %3, %11
  %12 = phi i256 [ %20, %11 ], [ %9, %3 ]
  %13 = phi i256 [ %19, %11 ], [ %8, %3 ]
  %14 = phi i256 [ %15, %11 ], [ %4, %3 ]
  %15 = mul nsw i256 %14, %14
  %16 = and i256 %12, 1
  %17 = icmp eq i256 %16, 0
  %18 = select i1 %17, i256 1, i256 %15
  %19 = mul nsw i256 %18, %13
  %20 = ashr i256 %12, 1
  %21 = icmp eq i256 %20, 0
  br i1 %21, label %22, label %11

22:                                               ; preds = %11, %3
  %23 = phi i256 [ %8, %3 ], [ %19, %11 ]
  store i256 %23, i256* %0, align 8
  ret void
}

; _ExtInt(256) ipow(_ExtInt(256) base, _ExtInt(256) exp)
; {
;     _ExtInt(256) result = 1;
;     while (1)
;     {
;         if (exp & (_ExtInt(256))1)
;             result *= base;
;         exp >>= (_ExtInt(256))1;
;         if (!exp)
;             break;
;         base *= base;
;     }
;     return result;
; }

; extern void f(_ExtInt(256) i);
; int main() {
;     _ExtInt(256) i = ipow((_ExtInt(256))0xFFFFFFFFFFFFFFFF, (_ExtInt(256))0xFFFFFFFFFFFFFFFF);
;     f(i);
;     return 0;

