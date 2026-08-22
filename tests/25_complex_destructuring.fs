fn main() -> int(32) {
    // 1. (x,y,z) = 10,20,30
    int(32) (x,y,z) = 10,20,30;
    log(x); // EXPECT: 10
    log(y); // EXPECT: 20
    log(z); // EXPECT: 30

    // 2. (xy1,xy2),z2 = 10,20
    int(32) (xy1,xy2),z2 = 10,20;
    log(xy1); // EXPECT: 10
    log(xy2); // EXPECT: 10
    log(z2);  // EXPECT: 20

    // 3. multiple uninitialized
    int(32) a,b,c;
    a = 1;
    b = 2;
    c = 3;
    log(a); // EXPECT: 1
    log(b); // EXPECT: 2
    log(c); // EXPECT: 3

    // 4. d,e,f = 10
    int(32) d,e,f = 10;
    log(d); // EXPECT: 10
    log(e); // EXPECT: 10
    log(f); // EXPECT: 10

    // 5. g,(h,i) = 10,20
    int(32) g,(h,i) = 10,20;
    log(g); // EXPECT: 10
    log(h); // EXPECT: 20
    log(i); // EXPECT: 20

    // 6. j,(k,l) = 10,(20,30)
    int(32) j,(k,l) = 10,(20,30);
    log(j); // EXPECT: 10
    log(k); // EXPECT: 20
    log(l); // EXPECT: 30

    // 7. array with empty brackets from list [1,2,3,4,5]
    int(32) arr[] = [1,2,3,4,5];
    log(arr[0]); // EXPECT: 1
    log(arr[4]); // EXPECT: 5

    return 0;
}
