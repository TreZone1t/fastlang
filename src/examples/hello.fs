fn greet(name: str) {
    log(name);
}

fn add(a: int(32), b: int(32)) -> int(32) {
    return a + b;
}

struct Point {
    public -> {
        int(32) x;
        int(32) y;
    }
    _(x: int(32), y: int(32)) ->{
        this.x = x;
        this.y = y;
    }
}

fn main() -> int(32) {
    greet("hello from Fast!");
    
    let int(32) result = add(10, 20);
    log(result);
    
    let int(32) a = 5;
    let int(32) b = 3;
    
    if (a > b) {
        log("a is greater");
    }
    
    let list nums = [1, 2, 3, 4, 5];
    let length len = nums;
    log(len);
    
    try -> {
        throw new error("test error handling");
    } catch(err) -> {
        log(err);
    }
    
    return 0;
}
