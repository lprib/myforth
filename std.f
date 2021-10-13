[stack operations]
intrinsic dup 'T -> 'T 'T;
intrinsic dup2 'T 'U -> 'T 'U 'T 'U;
intrinsic drop 'T -> ;
intrinsic over 'T 'U -> 'T 'U 'T;
intrinsic swap 'T 'U -> 'U 'T;
intrinsic rot 'T 'U 'V -> 'U 'V 'T;

[math]
intrinsic + i i -> i;
intrinsic - i i -> i;
intrinsic * i i -> i;
intrinsic / i i -> i;
intrinsic % i i -> i;
intrinsic >> i i -> i;
intrinsic << i i -> i;

[comparisons]
intrinsic < i i -> b;
intrinsic <= i i -> b;
intrinsic > i i -> b;
intrinsic >= i i -> b;
intrinsic = i i -> b;

[typecasts]
intrinsic (i) 'T -> i;
intrinsic (ui) 'T -> ui;
intrinsic (q) 'T -> q;
intrinsic (uq) 'T -> uq;
intrinsic (c) 'T -> c;
intrinsic (uc) 'T -> uc;
intrinsic (f) 'T -> f;
intrinsic (d) 'T -> d;
