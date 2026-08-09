let int(32) global_var = 100;

scope test -> {
    statement -> {
        let int(32) x = 10;
        
        // this inside a scope should pass
        set this.x -> 20;

        // global inside a scope should pass
        set global.global_var -> 50;
    }
}

// this in global scope should FAIL
// set this.global_var -> 10;

// global in global scope should warn
set global.global_var -> 99;
