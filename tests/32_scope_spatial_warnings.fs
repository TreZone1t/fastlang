// Test 32: Spatial Scope Awareness, Unused Variable/Parameter Warnings

fn calculate_area(width: int(32), height: int(32), unused_scale: float(32), _ignored_debug: bool) -> int(32) {
    int(32) area = width * height;
    int(32) unused_temp = 999;
    return area;
}

fn main() -> int(32) {
    int(32) result = calculate_area(10, 20, 1.5, true);
    log("Area result: ", result);
    return 0;
}
