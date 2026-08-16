// Valid: while and loop
fn sum_loop(count: int(32)) -> int(32) {
     int(32) i = 0;
     int(32) total = 0;
    while (i < count) -> {
         total = total + 1;
         i = i + 1;
    }
    //change the loop so make it infinite add don't have a count  condition
    loop ->{
    if (i < count) {
         total = total + 1;
         i = i + 1;
    }else{
        break;
    }
    }
    for (int(32) j = 0; j < 10; j = j + 1) -> {
     total = total + j;
    }
    return total;
}
