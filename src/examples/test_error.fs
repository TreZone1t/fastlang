scope main -> {
    type -> custom;
    flag -> {
        enable is_throw;
        enable all; // just to test
    }
    statement -> {
        try -> {
            log("I am going to throw!");
            throw new error("This is a custom error!");
        } catch (err) -> {
            log("Caught it:");
            log(err);
        }
    }
}
