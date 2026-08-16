fn main() -> int(32) {
    int(32) my_array[5] = [10, 20, 30, 40, 50];
    int(32) sum = 0;
    for (int(32) item in my_array) -> {
        sum = sum + item;
        log(item);
    }
    return sum;
}
