
; typedef _ExtInt(256) I;
; void udiv256(I n, I d, I* q) {
;     *q = 0;
;     while (n >= d) {
;         I i = 0, d_t = d;
;         while (n >= (d_t << 1) && ++i)
;             d_t <<= 1;
;         *q |= (I)1 << i;
;         n -= d_t;
;     }
; }
define dso_local void @udiv256(i256*, i256*, i256*) {
  store i256 0, i256* %2, align 8
  %4 = load i256, i256* %0, align 8
  %5 = load i256, i256* %1, align 8
  %6 = icmp slt i256 %4, %5
  br i1 %6, label %24, label %7

  %8 = phi i256 [ %22, %16 ], [ %5, %3 ]
  %9 = phi i256 [ %21, %16 ], [ %4, %3 ]
  br label %10

  %11 = phi i256 [ %15, %10 ], [ 0, %7 ]
  %12 = phi i256 [ %13, %10 ], [ %8, %7 ]
  %13 = shl i256 %12, 1
  %14 = icmp slt i256 %9, %13
  %15 = add nuw nsw i256 %11, 1
  br i1 %14, label %16, label %10

  %17 = shl nuw i256 1, %11
  %18 = load i256, i256* %2, align 8
  %19 = or i256 %18, %17
  store i256 %19, i256* %2, align 8
  %20 = load i256, i256* %0, align 8
  %21 = sub nsw i256 %20, %12
  store i256 %21, i256* %0, align 8
  %22 = load i256, i256* %1, align 8
  %23 = icmp slt i256 %21, %22
  br i1 %23, label %24, label %7

  ret void
}


; void sdiv256(I* n, I* d, I* q) {
;     I ret = (I)1;
;     if (*n < (I)0) { ret *= (I)-1; *n = -*n; }
;     if (*d < (I)0) { ret *= (I)-1; *d = -*d; }
;     udiv256(n, d, q);
;     *q *= ret;
; }
define dso_local void @sdiv256(i256*,i256*, i256*) {
  %4 = load i256, i256* %0, align 8
  %5 = icmp slt i256 %4, 0
  br i1 %5, label %6, label %8

  %7 = sub nsw i256 0, %4
  store i256 %7, i256* %0, align 8
  br label %8

  %9 = phi i256 [ -1, %6 ], [ 1, %3 ]
  %10 = load i256, i256* %1, align 8
  %11 = icmp slt i256 %10, 0
  br i1 %11, label %12, label %15

  %13 = sub nsw i256 0, %9
  %14 = sub nsw i256 0, %10
  store i256 %14, i256* %1, align 8
  br label %15

  %16 = phi i256 [ %13, %12 ], [ %9, %8 ]
  store i256 0, i256* %2, align 8
  %17 = load i256, i256* %0, align 8
  %18 = load i256, i256* %1, align 8
  %19 = icmp slt i256 %17, %18
  br i1 %19, label %39, label %20

  %21 = phi i256 [ %35, %29 ], [ %18, %15 ]
  %22 = phi i256 [ %34, %29 ], [ %17, %15 ]
  br label %23

  %24 = phi i256 [ %28, %23 ], [ 0, %20 ]
  %25 = phi i256 [ %26, %23 ], [ %21, %20 ]
  %26 = shl i256 %25, 1
  %27 = icmp slt i256 %22, %26
  %28 = add nuw nsw i256 %24, 1
  br i1 %27, label %29, label %23

  %30 = shl nuw i256 1, %24
  %31 = load i256, i256* %2, align 8
  %32 = or i256 %31, %30
  store i256 %32, i256* %2, align 8
  %33 = load i256, i256* %0, align 8
  %34 = sub nsw i256 %33, %25
  store i256 %34, i256* %0, align 8
  %35 = load i256, i256* %1, align 8
  %36 = icmp slt i256 %34, %35
  br i1 %36, label %37, label %20

  %38 = load i256, i256* %2, align 8
  br label %39

  %40 = phi i256 [ %38, %37 ], [ 0, %15 ]
  %41 = mul nsw i256 %40, %16
  store i256 %41, i256* %2, align 8
  ret void
}


; void neg(I* n) {
;     *n = -*n;
; }
define dso_local void @neg(i256*) {
  %2 = load i256, i256* %0, align 8
  %3 = sub nsw i256 0, %2
  store i256 %3, i256* %0, align 8
  ret void
}


; void modPow(I* b, I* e, I* ret) {
;     *ret = (I)1;
;     I p = *b;
;     for (I n = *e; n > (I)0; n >>= 1) {
;         if ((n & (I)1) != (I)0)
;             *ret *= p;
;         p *= p;
;     }
; }
define dso_local void @powmod(i256*, i256* ,i256*) {
  store i256 1, i256* %2, align 8
  %4 = load i256, i256* %1, align 8
  %5 = icmp sgt i256 %4, 0
  br i1 %5, label %6, label %8

  %7 = load i256, i256* %0, align 8
  br label %9

  ret void

  %10 = phi i256 [ %18, %17 ], [ 1, %6 ]
  %11 = phi i256 [ %20, %17 ], [ %4, %6 ]
  %12 = phi i256 [ %19, %17 ], [ %7, %6 ]
  %13 = and i256 %11, 1
  %14 = icmp eq i256 %13, 0
  br i1 %14, label %17, label %15

  %16 = mul nsw i256 %10, %12
  store i256 %16, i256* %2, align 8
  br label %17

  %18 = phi i256 [ %10, %9 ], [ %16, %15 ]
  %19 = mul nsw i256 %12, %12
  %20 = lshr i256 %11, 1
  %21 = icmp eq i256 %20, 0
  br i1 %21, label %8, label %9
}