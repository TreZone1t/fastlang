// 22_c_arrays.fs

int(32) arr[3] = [1, 2, 3];
float(64) arr2[3] = [1.1, 2.2, 3.3];
char arr3[6] = "hello";
bool arr4[3] = [true, false, true];

fn main() -> int(32) {
    int(32) x = arr[0];
    char c = arr3[1];
    log(x);
    log(c);
    return 0;
}
