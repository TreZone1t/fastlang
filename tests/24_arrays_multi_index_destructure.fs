custom Matrix -> {
    enable [ handle, data ];
    data -> 42;
    handle -> {
        fn index_access(r: int(32), c: int(32)) -> int(32) {
            return r * 10 + c;
        }
    }
}

fn sum_pair(arr: int(32)[]) -> int(32) {
    return arr[0] + arr[1];
}

fn main() -> int(32) {
    int(32) nums[] = [10, 20, 30];
    log(nums[0]);
    log(nums[1]);
    log(nums[2]);

    int(32) [first, second, third] = [100, 200, 300];
    log(first);
    log(second);
    log(third);

    Matrix mat = new Matrix();
    int(32) val = mat[3, 7];
    log(val);

    int(32) pair_sum = sum_pair([5, 15]);
    log(pair_sum);

    return 0;
}
// EXPECT: 10
// EXPECT: 20
// EXPECT: 30
// EXPECT: 100
// EXPECT: 200
// EXPECT: 300
// EXPECT: 37
// EXPECT: 20
