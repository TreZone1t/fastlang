class ComplexNumber -> {
    public -> {
        int(32) r = 0;
        int(32) i = 0;
    }

    handle -> {
        fn arrow_assign(arr: int(32)[]) -> void {
            this.r = arr[0];
            this.i = arr[1];
        }

        fn arrow(arr: int(32)[]) -> void {
            this.r = arr[0];
            this.i = arr[1];
        }

        fn add(other: ComplexNumber) -> ComplexNumber {
            ComplexNumber res;
            res.r = this.r + other.r;
            res.i = this.i + other.i;
            return res;
        }

        fn sub(other: ComplexNumber) -> ComplexNumber {
            ComplexNumber res;
            res.r = this.r - other.r;
            res.i = this.i - other.i;
            return res;
        }

        fn mul(other: ComplexNumber) -> ComplexNumber {
            ComplexNumber res;
            res.r = (this.r * other.r) - (this.i * other.i);
            res.i = (this.r * other.i) + (this.i * other.r);
            return res;
        }

        fn index_access(idx: int(32)) -> int(32) {
            if (idx == 0) {
                return this.r;
            }
            return this.i;
        }

        fn equal(other: ComplexNumber) -> bool {
            return (this.r == other.r) && (this.i == other.i);
        }

        fn not_equal(other: ComplexNumber) -> bool {
            return (this.r != other.r) || (this.i != other.i);
        }

        fn display() -> int(32) {
            return this.r + this.i;
        }
    }
}

fn main() -> int(32) {
    // 1. arrow_assign (declaration init)
    ComplexNumber c1 -> [3, 4];
    log(c1[0]); // EXPECT: 3
    log(c1[1]); // EXPECT: 4

    // 2. arrow (re-assignment)
    ComplexNumber c2;
    c2 -> [1, 2];
    log(c2[0]); // EXPECT: 1
    log(c2[1]); // EXPECT: 2

    // 3. add (+)
    ComplexNumber sum = c1 + c2;
    log(sum[0]); // EXPECT: 4
    log(sum[1]); // EXPECT: 6

    // 4. sub (-)
    ComplexNumber diff = c1 - c2;
    log(diff[0]); // EXPECT: 2
    log(diff[1]); // EXPECT: 2

    // 5. mul (*)
    ComplexNumber prod = c1 * c2;
    log(prod[0]); // EXPECT: -5
    log(prod[1]); // EXPECT: 10

    // 6. equal (==) and not_equal (!=)
    bool is_eq = (c1 == c2);
    log(is_eq); // EXPECT: 0

    bool is_neq = (c1 != c2);
    log(is_neq); // EXPECT: 1

    // 7. display
    log(sum); // EXPECT: 10

    return 0;
}
