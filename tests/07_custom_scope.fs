scope MathScope -> {
    type -> custom;
    enable [operators, index_access, data, handle, display];
    data -> 10;  
    
    handle -> {
        fn add(other: MathScope) -> MathScope {
            MathScope result -> new MathScope();
            result.data -> this.data + other.data;
            return result;
        }
        fn sub(other: MathScope) -> MathScope {
            MathScope result -> new MathScope();
            result.data -> this.data - other.data;
            return result;
        }
        fn mul(other: MathScope) -> MathScope {
            MathScope result -> new MathScope();
            result.data -> this.data * other.data;
            return result;
        }
        fn div(other: MathScope) -> MathScope {
            MathScope result -> new MathScope();
            result.data -> this.data / other.data;
            return result;
        }
        fn mod(other: MathScope) -> MathScope {
            MathScope result -> new MathScope();
            result.data -> this.data % other.data;
            return result;
        }
        fn index_access(index: int(32)) -> int(32) {
            return this.data + index;
        }
        fn display() -> string {
            return to_string(this.data);
        }
    }
}

scope Direction -> {
    type -> enum;
    variants -> {
        Up,
        Down,
        Left,
        Right
    };
    handle -> {
     fn display() -> string {
          switch (this) -> {
              case Up => { return "Up"; }
              case Down => { return "Down"; }
              case Left => { return "Left"; }
              case Right => { return "Right"; }
          }
     }
    }
}

fn main() -> int(32) {
  MathScope a -> new MathScope();
  MathScope b -> new MathScope();
  MathScope c -> a + b;
  MathScope d -> a - b;
  MathScope e -> a * b;
  MathScope f -> a / b;
  MathScope g -> a % b;
  log("c.data (10 + 10):");
  log(c.data);
  log("d.data (10 - 10):");
  log(d.data);
  log("e.data (10 * 10):");
  log(e.data);
  log("f.data (10 / 10):");
  log(f.data);
  log("g.data (10 % 10):");
  log(g.data);
  log("Index access a[5]:");
  log(a[5]);

  Direction dir1 -> Direction::Up;
  log(dir1);
  return 0;
}
// EXPECT: c.data (10 + 10):
// EXPECT: 20
// EXPECT: d.data (10 - 10):
// EXPECT: 0
// EXPECT: e.data (10 * 10):
// EXPECT: 100
// EXPECT: f.data (10 / 10):
// EXPECT: 1
// EXPECT: g.data (10 % 10):
// EXPECT: 0
// EXPECT: Index access a[5]:
// EXPECT: 15
// EXPECT: Up